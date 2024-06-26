use crate::physics::PhysicsSystem;
use crate::server::connections::SteadyMessageQueue;
use crate::server::lan::{ClientLanConnection, LanConnection, LanListener};
use crate::server::server_player::{ServerPlayer, ServerPlayerContainer};
use crate::worldmachine::ecs::{ComponentType, Entity, ParameterValue};
use crate::worldmachine::player::{MovementInfo, PlayerComponent};
use crate::worldmachine::throwballs::ThrowingBall;
use crate::worldmachine::{EntityId, WorldMachine, WorldUpdate};
use async_recursion::async_recursion;
use gfx_maths::*;
use halfbrown::HashMap;
use mutex_timeouts::tokio::MutexWithTimeoutAuto as Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant};
use tokio_util::codec::Encoder;

pub mod connections;
pub mod lan;
pub mod server_player;

pub type PacketUUID = String;
pub type ConnectionUUID = String;

#[derive(Clone)]
pub enum Connection {
    Local(Arc<LocalConnection>),
    Lan(LanListener, LanConnection),
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Connection::Local(_) => write!(f, "LocalConnection"),
            Connection::Lan(_, _) => write!(f, "LanConnection"),
        }
    }
}

#[derive(Clone)]
pub enum ConnectionClientside {
    Local(Arc<Mutex<LocalConnectionClientSide>>),
    Lan(ClientLanConnection),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FastPacket {
    ChangePosition(EntityId, Vec3),
    ChangeRotation(EntityId, Quaternion),
    ChangeScale(EntityId, Vec3),
    PlayerMoved(EntityId, Vec3, Quaternion, Quaternion),
    EntitySetParameter(EntityId, ComponentType, String, ParameterValue),
    PlayerMove(
        ConnectionUUID,
        Vec3,
        Vec3,
        Quaternion,
        Quaternion,
        Option<MovementInfo>,
    ),

    PlayerJump(ConnectionUUID),
    PlayerFuckYouMoveHere(Vec3),

    PlayerCheckPosition(ConnectionUUID, Vec3),

    PlayerFuckYouSetRotation(Quaternion),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FastPacketData {
    pub packet: Option<FastPacket>,
}

unsafe impl Send for FastPacketData {}

unsafe impl Sync for FastPacketData {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NameRejectionReason {
    IllegalWord,
    Taken,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SteadyPacket {
    InitialiseEntity(EntityId, Entity),
    RemoveEntity(EntityId),
    FinaliseMapLoad,
    InitialisePlayer(ConnectionUUID, EntityId, String, Vec3, Quaternion, Vec3),

    Message(String),
    ChatMessage(ConnectionUUID, String),
    SetName(ConnectionUUID, String),
    NameRejected(NameRejectionReason),
    Respawn(Vec3),
    ThrowThrowAballll(String, Vec3, Vec3),

    Ping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SteadyPacketData {
    pub packet: SteadyPacket,
    pub uuid: PacketUUID,
}

#[derive(Clone)]
pub struct LocalConnection {
    pub fast_update_sender: mpsc::Sender<FastPacketData>,
    pub steady_update_sender: mpsc::Sender<SteadyPacketData>,
    pub fast_update_receiver: Arc<Mutex<mpsc::Receiver<FastPacketData>>>,
    steady_update_receiver: Arc<Mutex<mpsc::Receiver<SteadyPacketData>>>,
    clientside_steady_update_sender: mpsc::Sender<SteadyPacketData>,
    pub consume_receiver_queue: Arc<Mutex<SteadyMessageQueue>>,
    pub uuid: ConnectionUUID,
}

pub struct LocalConnectionClientSide {
    pub fast_update_sender: mpsc::Sender<FastPacketData>,
    pub steady_update_sender: mpsc::Sender<SteadyPacketData>,
    pub steady_sender_queue: Arc<Mutex<SteadyMessageQueue>>,
    pub fast_update_receiver: mpsc::Receiver<FastPacketData>,
    pub steady_update_receiver: mpsc::Receiver<SteadyPacketData>,
}

#[derive(Clone)]
pub enum Connections {
    Local(Arc<Mutex<Vec<Arc<LocalConnection>>>>),
    Lan(LanListener, Arc<Mutex<Vec<LanConnection>>>),
}

#[derive(Clone)]
pub struct Server {
    pub connections: Connections,
    pub connections_incoming: Arc<Mutex<VecDeque<TcpStream>>>,
    pub worldmachine: Arc<Mutex<WorldMachine>>,
}

pub fn generate_uuid() -> PacketUUID {
    uuid::Uuid::new_v4().to_string()
}

impl Server {
    pub fn new(map_name: &str, physics: PhysicsSystem) -> Self {
        let mut worldmachine = WorldMachine::default();
        worldmachine.initialise(physics, true);
        worldmachine.load_map(map_name).expect("failed to load map");

        worldmachine.players = Some(Arc::new(Mutex::new(HashMap::new())));

        info!("server started");

        Self {
            connections: Connections::Local(Arc::new(Mutex::new(Vec::new()))),
            connections_incoming: Arc::new(Mutex::new(VecDeque::new())),
            worldmachine: Arc::new(Mutex::new(worldmachine)),
        }
    }

    pub async fn new_host_lan_server(
        map_name: &str,
        physics: PhysicsSystem,
        tcp_port: u16,
        udp_port: u16,
        hostname: &str,
    ) -> Self {
        let mut worldmachine = WorldMachine::default();
        worldmachine.initialise(physics, true);
        worldmachine.load_map(map_name).expect("failed to load map");

        worldmachine.players = Some(Arc::new(Mutex::new(HashMap::new())));

        let listener = LanListener::new(hostname, tcp_port, udp_port).await;

        let the_self = Self {
            connections: Connections::Lan(listener.clone(), Arc::new(Mutex::new(Vec::new()))),
            connections_incoming: Arc::new(Mutex::new(VecDeque::new())),
            worldmachine: Arc::new(Mutex::new(worldmachine)),
        };
        let the_clone = the_self.clone();
        let listener_clone = listener;
        tokio::spawn(async move {
            loop {
                the_clone
                    .connection_listening_thread(listener_clone.clone())
                    .await;
            }
        });

        info!("server started");
        the_self
    }

    async fn connection_listening_thread(&self, listener: LanListener) {
        loop {
            let new_connection = listener.poll_new_connection().await;
            if let Some(new_connection) = new_connection {
                self.connections_incoming
                    .lock()
                    .await
                    .push_back(new_connection);
            }
        }
    }

    pub async fn listen_for_lan_connections(&mut self) {
        if let Connections::Lan(listener, _connections_raw) = self.connections.clone() {
            let mut connections_incoming = self.connections_incoming.lock().await;
            while let Some(connection) = connections_incoming.pop_front() {
                let the_clone = self.clone();
                let listener_clone = listener.clone();
                tokio::spawn(async move {
                    let connection = listener_clone.clone().init_new_connection(connection).await;
                    if connection.is_none() {
                        return;
                    }
                    let connection = connection.unwrap();
                    let the_listener_clone = listener_clone.clone();
                    the_clone
                        .new_connection(Connection::Lan(
                            the_listener_clone.clone(),
                            connection.clone(),
                        ))
                        .await;
                });
                debug!("spawned new connection thread");
            }
        }
    }

    async fn get_connection_uuid(&self, connection: &Connection) -> ConnectionUUID {
        match connection {
            Connection::Local(local_connection) => local_connection.uuid.clone(),
            Connection::Lan(_, lan_connection) => lan_connection.uuid.clone(),
        }
    }

    async unsafe fn send_steady_packet_unsafe(
        &self,
        connection_og: &Connection,
        packet: SteadyPacketData,
    ) -> bool {
        match connection_og.clone() {
            Connection::Local(connection) => {
                let sus = connection.steady_update_sender.clone();
                sus.send(packet.clone()).await.unwrap();
                drop(sus);
                debug!("sent packet to local connection");
            }
            Connection::Lan(_, connection) => {
                let res = connection.serialise_and_send_steady(packet.clone()).await;
                return res.is_ok();
            }
        }
        true
    }

    async fn send_steady_packet(&self, connection: &Connection, packet: SteadyPacket) -> bool {
        match connection.clone() {
            Connection::Local(connection) => {
                let uuid = generate_uuid();
                let packet_data = SteadyPacketData {
                    packet,
                    uuid: uuid.clone(),
                };
                unsafe {
                    self.send_steady_packet_unsafe(&Connection::Local(connection), packet_data)
                        .await
                }
            }
            Connection::Lan(listener, connection) => {
                let uuid = generate_uuid();
                let packet_data = SteadyPacketData {
                    packet,
                    uuid: uuid.clone(),
                };
                unsafe {
                    self.send_steady_packet_unsafe(
                        &Connection::Lan(listener, connection),
                        packet_data,
                    )
                    .await
                }
            }
        }
    }

    pub async fn send_fast_packet(&self, connection: &Connection, packet: FastPacket) {
        match connection.clone() {
            Connection::Local(connection) => {
                let fus = connection.fast_update_sender.clone();
                let packet_data = FastPacketData {
                    packet: Some(packet),
                };
                fus.send(packet_data).await.unwrap();
            }
            Connection::Lan(listener, connection) => {
                let packet_data = FastPacketData {
                    packet: Some(packet),
                };
                connection
                    .serialise_and_send_fast(connection.uuid.clone(), listener.clone(), packet_data)
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn try_receive_fast_packet(&mut self, connection: &Connection) -> Option<FastPacket> {
        match connection.clone() {
            Connection::Local(connection) => {
                let mut fur = connection.fast_update_receiver.lock().await;
                if let Ok(packet) = fur.try_recv() {
                    return Some(packet.packet.unwrap());
                } else {
                    return None;
                }
            }
            Connection::Lan(listener, connection) => {
                let packet = connection
                    .attempt_receive_fast_and_deserialise(listener)
                    .await;
                if let Some(packet) = packet {
                    return Some(packet.packet.unwrap());
                }
            }
        }
        None
    }

    pub async fn begin_connection(&self, connection: Connection) -> Option<ServerPlayerContainer> {
        let worldmachine = self.worldmachine.lock().await;

        let world_clone = worldmachine.world.clone();
        let physics = worldmachine.physics.lock().unwrap().clone().unwrap();

        drop(worldmachine);
        for entity in world_clone.entities.iter() {
            let res = self
                .send_steady_packet(
                    &connection,
                    SteadyPacket::InitialiseEntity(entity.uid, entity.clone()),
                )
                .await;
            if !res {
                return None;
            }
        }
        debug!("sent all entity initialise packets");
        let uuid = self.get_connection_uuid(&connection).await;

        let name = "hardcoded muten";

        let position = Vec3::new(0.0, 2.0, 0.0);
        let rotation = Quaternion::identity();
        let scale = Vec3::new(1.0, 1.0, 1.0);

        let mut player = ServerPlayer::new(uuid.as_str(), name, position, rotation, scale);

        player.init(physics.clone()).await;

        let mut player_entity = Entity::new(player.name.lock().await.as_str());
        let entity_uuid = player_entity.uid;
        let player_component = PlayerComponent::new(name, uuid.clone(), position, rotation, scale);
        player_entity.add_component(player_component);

        let mut worldmachine = self.worldmachine.lock().await;
        worldmachine.world.entities.push(player_entity.clone());

        drop(worldmachine);
        let res = self
            .send_steady_packet(
                &connection,
                SteadyPacket::InitialisePlayer(
                    player.uuid.to_string(),
                    entity_uuid,
                    player.name.lock().await.clone(),
                    position,
                    rotation,
                    scale,
                ),
            )
            .await;
        if !res {
            return None;
        }
        debug!("sent player initialise packet");
        let mut worldmachine = self.worldmachine.lock().await;
        worldmachine.world.entities.push(player_entity.clone());
        worldmachine
            .queue_update(WorldUpdate::InitEntity(entity_uuid, player_entity.clone()))
            .await;

        let players = worldmachine.players.clone();
        drop(worldmachine);
        let players = players.unwrap();

        players.lock().await.insert(
            uuid.clone(),
            ServerPlayerContainer {
                player: player.clone(),
                entity_id: Some(entity_uuid),
                connection: connection.clone(),
            },
        );

        let res = self
            .send_steady_packet(&connection, SteadyPacket::FinaliseMapLoad)
            .await;

        if res {
            Some(players.lock().await.get(&uuid).cloned().unwrap())
        } else {
            None
        }
    }

    #[async_recursion]
    async fn steady_packet(&self, player: &ServerPlayerContainer, packet: SteadyPacket) -> bool {
        match packet {
            SteadyPacket::InitialiseEntity(_uid, _entity) => {
                debug!("client sent initialise packet");
            }
            SteadyPacket::InitialisePlayer(_, _, _, _, _, _) => {}
            SteadyPacket::Message(_) => {}
            SteadyPacket::FinaliseMapLoad => {}
            SteadyPacket::RemoveEntity(_) => {}
            SteadyPacket::ChatMessage(_who_sent, message) => {
                let who_sent = match player.connection.clone() {
                    Connection::Local(local_connection) => local_connection.uuid.clone(),
                    Connection::Lan(listener, connection) => connection.uuid.clone(),
                };
                let packet = SteadyPacket::ChatMessage(who_sent, message);
                match &self.connections {
                    Connections::Local(local_connections) => {
                        let cons = local_connections.lock().await.clone();
                        for connection in cons.iter() {
                            self.send_steady_packet(
                                &Connection::Local(connection.clone()),
                                packet.clone(),
                            )
                            .await;
                        }
                    }
                    Connections::Lan(listener, connections) => {
                        let cons = connections.lock().await.clone();
                        for a_connection in cons.iter() {
                            self.send_steady_packet(
                                &Connection::Lan(listener.clone(), a_connection.clone()),
                                packet.clone(),
                            )
                            .await;
                        }
                    }
                }
            }
            SteadyPacket::SetName(_who_sent, new_name) => {
                let who_sent = match player.connection.clone() {
                    Connection::Local(local_connection) => local_connection.uuid.clone(),
                    Connection::Lan(listener, connection) => connection.uuid.clone(),
                };
                let packet = SteadyPacket::SetName(who_sent, new_name.clone());
                match &self.connections {
                    Connections::Local(local_connections) => {
                        let cons = local_connections.lock().await.clone();
                        for connection in cons.iter() {
                            self.send_steady_packet(
                                &Connection::Local(connection.clone()),
                                packet.clone(),
                            )
                            .await;
                        }
                    }
                    Connections::Lan(listener, connections) => {
                        let mut name_taken = false;
                        let mut wm = self.worldmachine.lock().await;
                        let players = wm.players.as_mut().unwrap().lock().await;
                        for player in players.values() {
                            if *player.player.name.lock().await == new_name {
                                name_taken = true;
                                break;
                            }
                        }
                        drop(players);
                        drop(wm);
                        let connection = match &player.connection {
                            Connection::Local(_) => unreachable!(),
                            Connection::Lan(_, connection) => connection,
                        };
                        if name_taken {
                            self.send_steady_packet(
                                &Connection::Lan(listener.clone(), connection.clone()),
                                SteadyPacket::NameRejected(NameRejectionReason::Taken),
                            )
                            .await;
                        } else {
                            let mut wm = self.worldmachine.lock().await;
                            let mut players = wm.players.as_mut().unwrap().lock().await;
                            let player = players.get_mut(&connection.uuid).unwrap();
                            *player.player.name.lock().await = new_name.clone();
                            drop(players);
                            drop(wm);

                            let packet = SteadyPacket::SetName(connection.uuid.clone(), new_name);
                            let cons = connections.lock().await.clone();
                            for a_connection in cons {
                                self.send_steady_packet(
                                    &Connection::Lan(listener.clone(), a_connection.clone()),
                                    packet.clone(),
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            SteadyPacket::ThrowThrowAballll(_uuid, _positon, _initial_velocity) => {
                debug!("player threw snowball");
                let tball_cooldown = *player.player.tball_cooldown.lock().await;
                if tball_cooldown <= 0.0 {
                    *player.player.tball_cooldown.lock().await = 0.5;
                    let position = player.player.get_position(None, None).await;
                    let mut rotation = player.player.get_head_rotation(None, None).await;
                    rotation.w = -rotation.w;
                    let forward = rotation.forward();
                    let forward = Vec3::new(forward.x, forward.y, forward.z);
                    let position = forward * 1.5 + Vec3::new(0.0, 0.1, 0.0) + position;
                    let velocity = forward * 20.0 + Vec3::new(0.0, 5.0, 0.0);
                    let worldmachine = self.worldmachine.lock().await;
                    let physics = worldmachine.physics.clone();
                    drop(worldmachine);

                    let snowball = ThrowingBall::new(
                        position,
                        velocity,
                        physics.lock().unwrap().as_ref().unwrap(),
                    );

                    let packet =
                        SteadyPacket::ThrowThrowAballll(snowball.uuid.clone(), position, velocity);

                    let mut worldmachine = self.worldmachine.lock().await;
                    worldmachine.tballs.push(snowball);
                    drop(worldmachine);
                    match &self.connections {
                        Connections::Local(local_connections) => {
                            let cons = local_connections.lock().await.clone();
                            for connection in cons.iter() {
                                self.send_steady_packet(
                                    &Connection::Local(connection.clone()),
                                    packet.clone(),
                                )
                                .await;
                            }
                        }
                        Connections::Lan(listener, connections) => {
                            let cons = connections.lock().await.clone();
                            for a_connection in cons.iter() {
                                self.send_steady_packet(
                                    &Connection::Lan(listener.clone(), a_connection.clone()),
                                    packet.clone(),
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            SteadyPacket::Ping => {
                match &player.connection {
                    Connection::Local(local_connection) => {}
                    Connection::Lan(_, connection) => {
                        let unix_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        connection
                            .last_successful_ping
                            .store(unix_time, Ordering::Relaxed);
                    }
                };
            }
            SteadyPacket::NameRejected(_) => {}
            SteadyPacket::Respawn(_) => {}
        }
        true
    }

    async fn handle_steady_packets(
        &self,
        player: &ServerPlayerContainer,
        tcp_receiver: &mut Option<mpsc::Receiver<SteadyPacketData>>,
    ) -> bool {
        match player.connection.clone() {
            Connection::Local(local_connection) => {
                let sur = local_connection.steady_update_receiver.clone();
                let mut sur = sur.lock().await;
                if let Ok(packet) = sur.try_recv() {
                    drop(sur);
                    if !self.steady_packet(&player, packet.packet).await {
                        return false;
                    }
                }
            }
            Connection::Lan(_, lan_connection) => {
                let packet = lan_connection
                    .attempt_receive_steady_and_deserialise(tcp_receiver.as_mut().unwrap())
                    .await;
                if let Err(e) = packet {
                    debug!("error receiving steady packet: {:?}", e);
                    return false;
                }
                let packet_og = packet.unwrap();
                if let Some(packet) = packet_og.clone() {
                    if !self.steady_packet(&player, packet.packet).await {
                        return false;
                    }
                }
            }
        }
        true
    }

    async fn player_move(&self, player: &ServerPlayerContainer, packet: FastPacket) {
        if let FastPacket::PlayerMove(
            uuid,
            position,
            displacement_vector,
            rotation,
            head_rotation,
            movement_info,
        ) = packet
        {
            let (success, correct_position) = {
                player
                    .player
                    .attempt_position_change(
                        position,
                        displacement_vector,
                        rotation,
                        head_rotation,
                        movement_info.unwrap_or_default(),
                        player.entity_id,
                        self.worldmachine.clone(),
                    )
                    .await
            };
            if success {
            } else {
                self.send_fast_packet(
                    &player.connection,
                    FastPacket::PlayerFuckYouMoveHere(correct_position.unwrap()),
                )
                .await
            }
        }
    }

    async fn player_check_position(&self, player: &ServerPlayerContainer, packet: FastPacket) {
        if let FastPacket::PlayerCheckPosition(uuid, position) = packet {
            let worldmachine = self.worldmachine.clone();
            let mut worldmachine = worldmachine.lock().await;
            if !player.player.respawning.load(Ordering::Relaxed) {
                let server_position = player
                    .player
                    .get_position(player.entity_id, Some(&mut worldmachine))
                    .await;
                let success = server_position == position;
                if success {
                } else {
                    let position = player
                        .player
                        .get_position(player.entity_id, Some(&mut worldmachine))
                        .await;
                    drop(worldmachine);
                    self.send_fast_packet(
                        &player.connection,
                        FastPacket::PlayerFuckYouMoveHere(position),
                    )
                    .await
                }
            }
        }
    }

    async fn handle_fast_packets(&self, player: &ServerPlayerContainer) {
        match &player.connection {
            Connection::Local(local_connection) => {
                let mut fur = local_connection.fast_update_receiver.lock().await;
                if let Ok(packet) = fur.try_recv() {
                    drop(fur);
                    if let Some(fast_packet) = packet.packet {
                        match fast_packet.clone() {
                            FastPacket::PlayerMove(_, _, _, _, _, _) => {
                                self.player_move(player, fast_packet).await;
                            }

                            FastPacket::PlayerCheckPosition(_, _) => {
                                self.player_check_position(player, fast_packet).await;
                            }

                            FastPacket::PlayerJump(uuid) => {}

                            FastPacket::ChangePosition(_, _) => {}
                            FastPacket::ChangeRotation(_, _) => {}
                            FastPacket::ChangeScale(_, _) => {}
                            FastPacket::PlayerFuckYouMoveHere(_) => {}
                            FastPacket::PlayerFuckYouSetRotation(_) => {}
                            FastPacket::EntitySetParameter(_, _, _, _) => {}
                            FastPacket::PlayerMoved(_, _, _, _) => {}
                        }
                    }
                }
            }
            Connection::Lan(listener, lan_connection) => {
                let listener = listener.clone();
                let packet = lan_connection
                    .attempt_receive_fast_and_deserialise(listener)
                    .await;
                if let Some(packet) = packet {
                    match packet.clone().packet.unwrap() {
                        FastPacket::PlayerMove(_, _, _, _, _, _) => {
                            self.player_move(&player, packet.clone().packet.unwrap())
                                .await;
                        }

                        FastPacket::PlayerCheckPosition(_, _) => {
                            self.player_check_position(&player, packet.clone().packet.unwrap())
                                .await;
                        }

                        FastPacket::PlayerJump(uuid) => {}

                        FastPacket::ChangePosition(_, _) => {}
                        FastPacket::ChangeRotation(_, _) => {}
                        FastPacket::ChangeScale(_, _) => {}
                        FastPacket::PlayerFuckYouMoveHere(_) => {}
                        FastPacket::PlayerFuckYouSetRotation(_) => {}
                        FastPacket::EntitySetParameter(_, _, _, _) => {}
                        FastPacket::PlayerMoved(_, _, _, _) => {}
                    }
                }
            }
        }
    }

    pub async fn handle_connection(
        &self,
        connection: Connection,
        player: ServerPlayerContainer,
    ) -> bool {
        let mut tcp_receiver = match &connection {
            Connection::Local(_) => None,
            Connection::Lan(_, connection) => {
                connection.steady_receiver_passthrough.lock().await.take()
            }
        };
        loop {
            match connection.clone() {
                Connection::Local(local_connection) => {
                    self.handle_fast_packets(&player).await;
                    self.handle_steady_packets(&player, &mut None).await;
                }
                Connection::Lan(listener, lan_connection) => {
                    self.handle_fast_packets(&player).await;
                    let connection = self.handle_steady_packets(&player, &mut tcp_receiver).await;
                    if !connection {
                        return false;
                    }
                }
            }
        }
    }

    async fn assert_connection_type_allowed(&self, connection: Connection) -> bool {
        match connection {
            Connection::Local(_) => {
                matches!(self.connections.clone(), Connections::Local(_))
            }
            Connection::Lan(_, _) => {
                matches!(self.connections.clone(), Connections::Lan(_, _))
            }
        }
    }

    async fn disconnect_player(&self, uuid: ConnectionUUID, player_entity_id: EntityId) {
        let connections = match self.connections.clone() {
            Connections::Lan(_, connections) => connections.clone(),
            _ => {
                panic!("assert_connection_type_allowed failed");
            }
        };
        let mut connections = connections.lock().await;
        connections.retain(|x| x.uuid != uuid);
        debug!("connections: {:?}", connections.len());
        drop(connections);

        let worldmachine = self.worldmachine.clone();
        let mut worldmachine = worldmachine.lock().await;
        if worldmachine
            .world
            .entities
            .iter()
            .any(|x| x.uid == player_entity_id)
        {
            worldmachine
                .world
                .entities
                .retain(|x| x.uid != player_entity_id);
            worldmachine
                .queue_update(WorldUpdate::EntityNoLongerExists(player_entity_id))
                .await;
        }
        let players = worldmachine.players.clone();
        drop(worldmachine);
        if let Some(players) = players {
            let mut players = players.lock().await;
            players.retain(|_, x| x.entity_id != Some(player_entity_id));
        }
    }

    async fn new_connection(&self, connection: Connection) {
        if self
            .assert_connection_type_allowed(connection.clone())
            .await
        {
            match connection.clone() {
                Connection::Local(local_connection) => {
                    let _connection_index = match self.connections.clone() {
                        Connections::Local(connections) => {
                            let mut connections = connections.lock().await;
                            connections.push(local_connection.clone());
                            connections.len() - 1
                        }
                        _ => {
                            panic!("assert_connection_type_allowed failed");
                        }
                    };
                    let player = self.begin_connection(connection.clone()).await;
                    self.handle_connection(connection, player.unwrap()).await;
                }
                Connection::Lan(_, lan_connection) => {
                    let _connection_index = match self.connections.clone() {
                        Connections::Lan(_, connections) => {
                            let mut connections = connections.lock().await;
                            connections.push(lan_connection.clone());
                            connections.len() - 1
                        }
                        _ => {
                            panic!("assert_connection_type_allowed failed");
                        }
                    };
                    let player = self.begin_connection(connection.clone()).await;
                    if player.is_none() {
                        let connections = match self.connections.clone() {
                            Connections::Lan(_, connections) => connections.clone(),
                            _ => {
                                panic!("assert_connection_type_allowed failed");
                            }
                        };
                        let mut connections = connections.lock().await;
                        connections.retain(|x| x.uuid != lan_connection.uuid);
                        debug!("connections: {:?}", connections.len());
                        return;
                    }
                    let player = player.unwrap();
                    let entity_id = player.entity_id.unwrap();
                    let connected = self.handle_connection(connection, player).await;
                    if !connected {
                        self.disconnect_player(lan_connection.uuid, entity_id).await;
                    }
                }
            }
        }
    }

    pub async fn join_local_server(&mut self) -> Arc<Mutex<LocalConnectionClientSide>> {
        info!("joining local server");
        let (fast_update_sender_client, fast_update_receiver_server) = mpsc::channel(100);
        let (steady_update_sender_client, steady_update_receiver_server) = mpsc::channel(100);
        let (fast_update_sender_server, fast_update_receiver_client) = mpsc::channel(100);
        let (steady_update_sender_server, steady_update_receiver_client) = mpsc::channel(100);
        let uuid = generate_uuid();
        let local_connection = LocalConnection {
            fast_update_sender: fast_update_sender_server,
            steady_update_sender: steady_update_sender_server,
            fast_update_receiver: Arc::new(Mutex::new(fast_update_receiver_server)),
            steady_update_receiver: Arc::new(Mutex::new(steady_update_receiver_server)),
            clientside_steady_update_sender: steady_update_sender_client.clone(),
            consume_receiver_queue: Arc::new(Mutex::new(SteadyMessageQueue::new())),
            uuid,
        };
        let local_connection_client_side = LocalConnectionClientSide {
            fast_update_sender: fast_update_sender_client,
            steady_update_sender: steady_update_sender_client,
            steady_sender_queue: Arc::new(Mutex::new(SteadyMessageQueue::new())),
            fast_update_receiver: fast_update_receiver_client,
            steady_update_receiver: steady_update_receiver_client,
        };
        struct ThreadData {
            server: Server,
            connection: Arc<Mutex<LocalConnectionClientSide>>,
        }
        let connection = Arc::new(Mutex::new(local_connection_client_side));
        let thread_data = ThreadData {
            server: self.clone(),
            connection: connection.clone(),
        };
        tokio::spawn(async move {
            let thread_data = thread_data;
            let connection = Arc::new(local_connection);
            thread_data
                .server
                .new_connection(Connection::Local(connection))
                .await;
        });
        connection
    }

    async fn get_connections_affected_from_position(&mut self, position: Vec3) -> Vec<Connection> {
        let mut connections_affected = Vec::new();
        match self.connections.clone() {
            Connections::Local(connections) => {
                let connections = connections.lock().await;
                for connection in connections.iter() {
                    connections_affected.push(Connection::Local(connection.clone()));
                }
            }
            Connections::Lan(listener, connections) => {
                let connections = connections.lock().await;
                for connection in connections.iter() {
                    connections_affected
                        .push(Connection::Lan(listener.clone(), connection.clone()));
                }
            }
        }
        connections_affected
    }

    async fn get_all_connections(&mut self) -> Vec<Connection> {
        let mut connections_final = Vec::new();
        match self.connections.clone() {
            Connections::Local(connections) => {
                let connections = connections.lock().await;
                for connection in connections.iter() {
                    connections_final.push(Connection::Local(connection.clone()));
                }
            }
            Connections::Lan(listener, connections) => {
                let connections = connections.lock().await;
                for connection in connections.iter() {
                    connections_final.push(Connection::Lan(listener.clone(), connection.clone()));
                }
            }
        }
        connections_final
    }

    pub async fn handle_world_updates(&mut self, updates: Vec<WorldUpdate>) {
        let mut player_entity_movement_stack: HashMap<
            EntityId,
            Vec<(Vec3, Quaternion, Quaternion)>,
        > = HashMap::new();
        for update in updates {
            match update {
                WorldUpdate::SetPosition(entity_id, vec3) => {
                    let connections = self.get_connections_affected_from_position(vec3).await;
                    for connection in connections {
                        self.send_fast_packet(
                            &connection,
                            FastPacket::ChangePosition(entity_id, vec3),
                        )
                        .await;
                    }
                }
                WorldUpdate::SetRotation(entity_id, quat) => {
                    let connections = self
                        .get_connections_affected_from_position(Vec3::new(0.0, 0.0, 0.0))
                        .await;
                    for connection in connections {
                        self.send_fast_packet(
                            &connection,
                            FastPacket::ChangeRotation(entity_id, quat),
                        )
                        .await;
                    }
                }
                WorldUpdate::SetScale(entity_id, vec3) => {
                    let connections = self
                        .get_connections_affected_from_position(Vec3::new(0.0, 0.0, 0.0))
                        .await;
                    for connection in connections {
                        self.send_fast_packet(
                            &connection,
                            FastPacket::ChangeScale(entity_id, vec3),
                        )
                        .await;
                    }
                }
                WorldUpdate::InitEntity(entity_id, entity_data) => {
                    let connections = self.get_all_connections().await;
                    for connection in connections {
                        self.send_steady_packet(
                            &connection,
                            SteadyPacket::InitialiseEntity(entity_id, entity_data.clone()),
                        )
                        .await;
                    }
                }
                WorldUpdate::EntityNoLongerExists(entity_id) => {
                    let connections = self.get_all_connections().await;
                    for connection in connections {
                        self.send_steady_packet(&connection, SteadyPacket::RemoveEntity(entity_id))
                            .await;
                    }
                }
                WorldUpdate::MovePlayerEntity(entity_id, position, rotation, head_rotation) => {
                    player_entity_movement_stack
                        .entry(entity_id)
                        .or_insert(Vec::new())
                        .push((position, rotation, head_rotation));
                }
            }
        }

        for (entity_id, movement_stack) in player_entity_movement_stack {
            let movement = movement_stack.last().unwrap();
            let connections = self
                .get_connections_affected_from_position(movement.0)
                .await;
            for connection in connections {
                self.send_fast_packet(
                    &connection,
                    FastPacket::PlayerMoved(entity_id, movement.0, movement.1, movement.2),
                )
                .await;
            }
        }
    }

    pub async fn physics_thread(&self) {
        loop {}
    }

    pub async fn player_and_physics_tick_thread(&self) {
        let mut compensation_delta = 0.0;
        let mut delta;
        loop {
            {
                let worldmachine = self.worldmachine.lock().await;
                let last_physics_tick = worldmachine.last_physics_update;
                drop(worldmachine);
                let current_time = std::time::Instant::now();
                delta = (current_time - last_physics_tick).as_secs_f32();
                if delta > 0.01 {
                    let mut worldmachine = self.worldmachine.lock().await;
                    let res = worldmachine
                        .physics
                        .lock()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .tick(delta + compensation_delta);
                    if let Some(delta) = res {
                        compensation_delta += delta;
                    } else {
                        compensation_delta = 0.0;
                        worldmachine.last_physics_update = current_time;
                    }
                }
            }
            {
                let worldmachine = self.worldmachine.lock().await;
                let players = worldmachine.players.clone().unwrap();
                drop(worldmachine);
                let mut players_to_disconnect = Vec::new();
                let mut players = players.lock().await.clone();
                for (_uuid, player) in players.iter_mut() {
                    let last_ping = match &player.connection {
                        Connection::Local(_) => SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        Connection::Lan(_, con) => con.last_successful_ping.load(Ordering::Relaxed),
                    };
                    let current_time = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if current_time - last_ping > 60 {
                        players_to_disconnect
                            .push((player.connection.clone(), player.entity_id.unwrap()));
                        continue;
                    }
                    if player.player.gravity_tick().await {
                        let mut worldmachine = self.worldmachine.lock().await;
                        let _pos = player
                            .player
                            .get_position(player.entity_id, Some(&mut worldmachine))
                            .await;
                        drop(worldmachine);
                    }
                    *player.player.tball_cooldown.lock().await -= delta;
                    let position = player.player.get_position(None, None).await;
                    if position.y < -20.0 {
                        player.player.respawning.store(true, Ordering::Relaxed);
                        let respawning = player.player.respawning.clone();
                        let packet = SteadyPacket::Respawn(Vec3::new(0.0, 0.0, 0.0));
                        self.send_steady_packet(&player.connection, packet).await;
                        let mut worldmachine = self.worldmachine.lock().await;
                        player
                            .player
                            .set_position(
                                Vec3::new(0.0, 0.0, 0.0),
                                player.entity_id,
                                &mut worldmachine,
                            )
                            .await;
                        drop(worldmachine);
                        respawning.store(false, Ordering::Relaxed);
                    }
                }
                drop(players);
                for player in players_to_disconnect {
                    match player.0 {
                        Connection::Local(con) => {
                            self.disconnect_player(con.uuid.clone(), player.1).await;
                        }
                        Connection::Lan(_, con) => {
                            self.disconnect_player(con.uuid.clone(), player.1).await;
                        }
                    }
                }
            }
        }
    }

    pub async fn run(&mut self) {
        loop {
            {
                let mut worldmachine = self.worldmachine.lock().await;
                let updates = { worldmachine.server_tick().await };
                drop(worldmachine);
                if let Some(updates) = updates {
                    self.handle_world_updates(updates).await;
                }
            }

            self.listen_for_lan_connections().await;
        }
    }
}
