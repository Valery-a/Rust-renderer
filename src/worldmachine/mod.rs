use fyrox_sound::context::SoundContext;
use gfx_maths::{ Quaternion, Vec3 };
use halfbrown::HashMap;
use serde::{ Deserialize, Serialize };
use std::borrow::{ Borrow, BorrowMut };
use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crate::audio::AudioBackend;
use crate::common_anim::animation_move::MoveAnim;
use crate::physics::{ Materials, PhysicsSystem };
use crate::server::server_player::ServerPlayerContainer;
use crate::server::{
    ConnectionClientside,
    ConnectionUUID,
    FastPacket,
    FastPacketData,
    NameRejectionReason,
    SteadyPacket,
    SteadyPacketData,
};
use crate::ui_defs::chat;
use crate::worldmachine::components::{
    COMPONENT_TYPE_BOX_COLLIDER,
    COMPONENT_TYPE_JUKEBOX,
    COMPONENT_TYPE_LIGHT,
    COMPONENT_TYPE_MESH_RENDERER,
    COMPONENT_TYPE_PLAYER,
    COMPONENT_TYPE_TERRAIN,
    COMPONENT_TYPE_TRANSFORM,
    COMPONENT_TYPE_TRIGGER,
};
use crate::worldmachine::ecs::*;
use crate::worldmachine::player::{ MovementInfo, Player, PlayerContainer };
use crate::worldmachine::MapLoadError::FolderNotFound;
use crate::{ server, MutRenderer };
use mutex_timeouts::tokio::MutexWithTimeoutAuto as Mutex;
use tokio::sync::mpsc::error::TryRecvError;

use self::throwballs::ThrowingBall;

pub mod components;
pub mod ecs;
pub mod entities;
pub mod helpers;
pub mod player;
pub mod throwballs;

pub type EntityId = u64;

#[derive(Deserialize, Serialize)]
pub struct World {
    pub entities: Vec<Entity>,
    pub systems: Vec<System>,
    eid_manager: EntityId,
    current_map: String,
}

#[derive(Deserialize, Serialize)]
pub struct WorldDef {
    pub name: String,
    pub world: World,
}

#[derive(Clone, Debug)]
pub enum WorldUpdate {
    InitEntity(EntityId, Entity),
    SetPosition(EntityId, Vec3),
    SetRotation(EntityId, Quaternion),
    SetScale(EntityId, Vec3),
    MovePlayerEntity(EntityId, Vec3, Quaternion, Quaternion),
    EntityNoLongerExists(EntityId),
}

#[derive(Clone, Debug)]
pub enum ClientUpdate {
    IDisplaced((Vec3, Option<MovementInfo>)),
    ILooked(Quaternion),

    IMoved(Vec3, Option<Vec3>, Quaternion, Quaternion, Option<MovementInfo>),
    IJumped,
    IThrewtball,
}

#[derive(Clone, Debug)]
pub enum MapLoadError {
    FolderNotFound(String),
}

impl Clone for World {
    fn clone(&self) -> Self {
        let mut entities = Vec::new();
        for entity in &self.entities {
            entities.push(entity.deref().clone());
        }
        let mut systems = Vec::new();
        for system in &self.systems {
            systems.push(system.deref().clone());
        }
        World {
            entities,
            systems,
            eid_manager: 0,
            current_map: self.current_map.clone(),
        }
    }
}

pub struct WorldMachine {
    pub world: World,
    pub tballs: Vec<ThrowingBall>,
    pub physics: Arc<mutex_timeouts::std::MutexWithTimeout<Option<PhysicsSystem>>>,
    pub last_physics_update: std::time::Instant,
    pub game_data_path: String,
    pub counter: f32,
    pub entities_wanting_to_load_things: Vec<usize>,
    pub command: String,
    lights_changed: bool,
    is_server: bool,
    server_connection: Option<crate::server::ConnectionClientside>,
    world_update_queue: Arc<Mutex<VecDeque<WorldUpdate>>>,
    client_update_queue: Arc<Mutex<VecDeque<ClientUpdate>>>,
    pub player: Option<PlayerContainer>,
    ignore_this_entity: Option<EntityId>,
    pub players: Option<Arc<Mutex<HashMap<ConnectionUUID, ServerPlayerContainer>>>>,

    last_ping: Instant,
}

impl Default for WorldMachine {
    fn default() -> Self {
        let world = World {
            entities: Vec::new(),
            systems: Vec::new(),
            eid_manager: 0,
            current_map: "".to_string(),
        };
        Self {
            world,
            tballs: vec![],
            physics: Arc::new(mutex_timeouts::std::MutexWithTimeout::new(None)),
            last_physics_update: std::time::Instant::now(),
            game_data_path: String::from(""),
            counter: 0.0,
            command: String::new(),
            entities_wanting_to_load_things: Vec::new(),
            lights_changed: true,
            is_server: false,
            server_connection: None,
            world_update_queue: Arc::new(Mutex::new(VecDeque::new())),
            client_update_queue: Arc::new(Mutex::new(VecDeque::new())),
            player: None,
            ignore_this_entity: None,
            players: None,
            last_ping: Instant::now(),
        }
    }
}

impl WorldMachine {
    pub fn initialise(&mut self, physics: PhysicsSystem, is_server: bool) {
        let _ = *components::COMPONENTS_INITIALISED;
        self.game_data_path = String::from("base");
        self.physics = Arc::new(mutex_timeouts::std::MutexWithTimeout::new(Some(physics)));
        self.is_server = is_server;

        if self.is_server {
            let physics = self.physics.lock().unwrap().as_mut().unwrap().copy_with_new_scene();
            self.physics = Arc::new(mutex_timeouts::std::MutexWithTimeout::new(Some(physics)));
        }

        self.blank_slate(is_server);
    }

    pub fn blank_slate(&mut self, is_server: bool) {
        {
            let mut eid_manager = ENTITY_ID_MANAGER.lock().unwrap();
            eid_manager.borrow_mut().id = 0;
        }
        self.world.entities.clear();
        self.world.systems.clear();
        self.counter = 0.0;
        self.lights_changed = true;
    }

    pub fn load_map(&mut self, map_name: &str) -> Result<(), MapLoadError> {
        self.blank_slate(self.is_server);
        let map_dir = format!("{}/maps/{}", self.game_data_path, map_name);
        if !std::path::Path::new(&map_dir).exists() {
            return Err(FolderNotFound(map_dir));
        }
        let mut deserializer = rmp_serde::Deserializer::new(
            std::fs::File::open(format!("{}/worlddef", map_dir)).unwrap()
        );
        let world_def: WorldDef = Deserialize::deserialize(&mut deserializer).unwrap();

        for entity in world_def.world.entities {
            let mut entity_new = unsafe { Entity::new(entity.name.as_str()) };
            for component in entity.components {
                let component_type = ComponentType::get(component.get_type().name);
                if component_type.is_none() {
                    panic!("component type not found: {}", component.get_type().name);
                }
                let component_type = component_type.unwrap();
                let mut component = component;
                component.component_type = component_type.clone();

                entity_new.add_component(component);
            }
            self.world.entities.push(entity_new);
        }

        self.world.current_map = map_name.to_string();

        self.initialise_entities();

        if self.is_server {
            let mut entity_init_packets = Vec::new();
            for entity in &self.world.entities {
                entity_init_packets.push(WorldUpdate::InitEntity(entity.uid, entity.clone()));
            }
            self.queue_updates(entity_init_packets);
        }

        for system in world_def.world.systems {
            self.world.systems.push(system);
        }

        Ok(())
    }

    pub fn initialise_entities(&mut self) {
        for entity in &mut self.world.entities {
            if let Some(box_collider) = entity.get_component(COMPONENT_TYPE_BOX_COLLIDER.clone()) {
                let box_collider = box_collider.borrow();
                let position = box_collider.get_parameter("position").borrow().clone();
                let mut position = match position.value {
                    ParameterValue::Vec3(position) => position,
                    _ => panic!("position is not a vec3"),
                };
                let scale = box_collider.get_parameter("scale").borrow().clone();
                let mut scale = match scale.value {
                    ParameterValue::Vec3(scale) => scale,
                    _ => panic!("scale is not a vec3"),
                };
                if let Some(transform) = entity.get_component(COMPONENT_TYPE_TRANSFORM.clone()) {
                    let transform = transform.borrow();
                    let trans_position = transform.get_parameter("position").borrow().clone();
                    let trans_position = match trans_position.value {
                        ParameterValue::Vec3(position) => position,
                        _ => panic!("position is not a vec3"),
                    };
                    let trans_scale = transform.get_parameter("scale").borrow().clone();
                    let trans_scale = match trans_scale.value {
                        ParameterValue::Vec3(scale) => scale,
                        _ => panic!("scale is not a vec3"),
                    };
                    position += trans_position;
                    scale *= trans_scale;
                }
                let box_collider_physics = self.physics
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .create_box_collider_static(position, scale, Materials::Player)
                    .unwrap();
                box_collider_physics.add_self_to_scene(
                    self.physics.lock().unwrap().clone().unwrap()
                );
            }
            if let Some(trigger) = entity.get_component(COMPONENT_TYPE_TRIGGER.clone()) {
                let trigger = trigger.borrow();
                let position = trigger.get_parameter("position").borrow().clone();
                let mut position = match position.value {
                    ParameterValue::Vec3(position) => position,
                    _ => panic!("position is not a vec3"),
                };
                let scale = trigger.get_parameter("size").borrow().clone();
                let mut scale = match scale.value {
                    ParameterValue::Vec3(scale) => scale,
                    _ => panic!("scale is not a vec3"),
                };
                if let Some(transform) = entity.get_component(COMPONENT_TYPE_TRANSFORM.clone()) {
                    let transform = transform.borrow();
                    let trans_position = transform.get_parameter("position").borrow().clone();
                    let trans_position = match trans_position.value {
                        ParameterValue::Vec3(position) => position,
                        _ => panic!("position is not a vec3"),
                    };
                    let trans_scale = transform.get_parameter("scale").borrow().clone();
                    let trans_scale = match trans_scale.value {
                        ParameterValue::Vec3(scale) => scale,
                        _ => panic!("scale is not a vec3"),
                    };
                    position += trans_position;
                    scale *= trans_scale;
                }
                let trigger_physics = self.physics
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .create_trigger_shape(position, scale, Materials::Player)
                    .unwrap();
                trigger_physics.add_self_to_scene(self.physics.lock().unwrap().clone().unwrap());
                debug!(
                    "added trigger to physics scene with position: {:?} and scale: {:?}",
                    position,
                    scale
                );
            }
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_entity(&self, entity_id: EntityId) -> Option<Arc<Mutex<&Entity>>> {
        for entity in self.world.entities.iter() {
            if entity.get_id() == entity_id {
                return Some(Arc::new(Mutex::new(entity)));
            }
        }
        None
    }

    pub fn get_entity_index(&self, entity_id: EntityId) -> Option<usize> {
        for (index, entity) in self.world.entities.iter().enumerate() {
            if entity.get_id() == entity_id {
                return Some(index);
            }
        }
        None
    }

    pub fn remove_entity_at_index(&mut self, index: usize) {
        self.world.entities.remove(index);
    }

    pub fn send_lights_to_renderer(&mut self) -> Option<Vec<crate::light::Light>> {
        let mut lights = Vec::new();
        for entity in &self.world.entities {
            let components = entity.get_components();
            let mut light_component = None;
            let mut transform_component = None;
            for component in components {
                if component.get_type() == COMPONENT_TYPE_LIGHT.clone() {
                    light_component = Some(component);
                }
                if component.get_type() == COMPONENT_TYPE_TRANSFORM.clone() {
                    transform_component = Some(component);
                }
            }
            if let Some(light) = light_component {
                let light = light.clone();
                let position = light.get_parameter("position");
                let mut position = match position.value {
                    ParameterValue::Vec3(v) => v,
                    _ => {
                        error!("send_lights_to_renderer: light position is not a vec3");
                        Vec3::new(0.0, 0.0, 0.0)
                    }
                };
                let color = light.get_parameter("colour");
                let color = match color.value {
                    ParameterValue::Vec3(v) => v,
                    _ => {
                        error!("send_lights_to_renderer: light color is not a vec3");
                        Vec3::new(0.0, 0.0, 0.0)
                    }
                };
                let intensity = light.get_parameter("intensity");
                let intensity = match intensity.value {
                    ParameterValue::Float(v) => v,
                    _ => {
                        error!("send_lights_to_renderer: light intensity is not a float");
                        0.0
                    }
                };
                let radius = light.get_parameter("radius");
                let radius = match radius.value {
                    ParameterValue::Float(v) => v,
                    _ => {
                        error!("send_lights_to_renderer: light radius is not a float");
                        0.0
                    }
                };
                let casts_shadow = light.get_parameter("casts_shadow");
                let casts_shadow = match casts_shadow.value {
                    ParameterValue::Bool(v) => v,
                    _ => {
                        error!("send_lights_to_renderer: light casts_shadow is not a bool");
                        false
                    }
                };

                if let Some(transform) = transform_component {
                    let transform = transform.clone();
                    let trans_position = transform.get_parameter("position");
                    let trans_position = match trans_position.value {
                        ParameterValue::Vec3(v) => v,
                        _ => {
                            error!("send_lights_to_renderer: transform position is not a vec3");
                            Vec3::new(0.0, 0.0, 0.0)
                        }
                    };
                    position += trans_position;
                }
                lights.push(crate::light::Light {
                    position,
                    color,
                    intensity: intensity as f32,
                    radius: radius as f32,
                    casts_shadow,
                });
            }
        }
        self.lights_changed = false;
        Some(lights)
    }

    pub fn connect_to_server(&mut self, connection: ConnectionClientside) {
        self.server_connection = Some(connection);
    }

    async fn send_fast_message(&mut self, message: FastPacketData) {
        if let Some(connection) = &mut self.server_connection {
            match connection {
                ConnectionClientside::Local(connection) => {
                    let mut connection = connection.lock().await;
                    let attempt = connection.fast_update_sender.send(message).await;
                    if attempt.is_err() {
                        error!("send_fast_message: failed to send message");
                    }
                }
                ConnectionClientside::Lan(connection) => {
                    let attempt = connection.send_fast_and_serialise(message).await;
                }
            }
        }
    }

    async fn send_steady_message(&mut self, message: SteadyPacketData) -> bool {
        if let Some(connection) = &mut self.server_connection {
            match connection {
                ConnectionClientside::Local(connection) => {
                    let mut connection = connection.lock().await;
                    let attempt = connection.steady_update_sender.send(message).await;
                    if attempt.is_err() {
                        error!("send_steady_message: failed to send message");
                        return false;
                    }
                    true
                }
                ConnectionClientside::Lan(connection) => {
                    let attempt = connection.send_steady_and_serialise(message).await;
                    if attempt.is_err() {
                        error!("send_steady_message: failed to send message");
                        return false;
                    }
                    true
                }
            }
        } else {
            false
        }
    }

    async fn initialise_entity(&mut self, packet: SteadyPacket) {
        if let SteadyPacket::InitialiseEntity(entity_id, entity_data) = packet {
        }
    }

    async fn initialise_player(&mut self, packet: SteadyPacket) {
        if let SteadyPacket::InitialisePlayer(uuid, id, name, position, rotation, scale) = packet {
        }
    }

    async fn remove_entity(&mut self, packet: SteadyPacket) {
        if let SteadyPacket::RemoveEntity(entity_id) = packet {
        }
    }

    pub async fn set_name(&mut self, name: String) {
        self.send_steady_message(SteadyPacketData {
            packet: SteadyPacket::SetName(String::new(), name),
            uuid: server::generate_uuid(),
        }).await;
    }

    pub async fn send_chat_message(&mut self, message: String) {
        self.send_steady_message(SteadyPacketData {
            packet: SteadyPacket::ChatMessage(String::new(), message),
            uuid: server::generate_uuid(),
        }).await;
    }

    pub async fn throw_tball(&mut self) {
        self.send_steady_message(SteadyPacketData {
            packet: SteadyPacket::ThrowThrowAballll(
                String::new(),
                Vec3::default(),
                Vec3::default()
            ),
            uuid: server::generate_uuid(),
        }).await;
    }

    async fn handle_steady_message(&mut self, packet: SteadyPacket) {
        match packet {
            SteadyPacket::InitialiseEntity(entity_id, entity_data) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }

                if let Some(_player) = entity_data.get_component(COMPONENT_TYPE_PLAYER.clone()) {
                    chat::write_chat("engine".to_string(), "a new player has joined!".to_string());
                }

                if self.get_entity(entity_id).is_none() {
                    let mut entity = unsafe {
                        Entity::new_with_id(entity_data.name.as_str(), entity_id)
                    };
                    entity.copy_data_from_other_entity(&entity_data);
                    self.world.entities.push(entity);
                    self.entities_wanting_to_load_things.push(self.world.entities.len() - 1);
                } else {
                    let entity_index = self.get_entity_index(entity_id).unwrap();
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    entity.copy_data_from_other_entity(&entity_data);
                    self.entities_wanting_to_load_things.push(entity_index);
                }
                debug!("initialise entity message received");
            }
            SteadyPacket::Message(str_message) => {
                info!("Received message from server: {}", str_message);
            }
            SteadyPacket::InitialisePlayer(uuid, id, name, position, rotation, scale) => {
                debug!("initialise player message received");
                let mut player = Player::default();
                player.init(
                    self.physics.lock().unwrap().clone().unwrap(),
                    uuid,
                    name.clone(),
                    position,
                    rotation,
                    scale
                );
                chat::CHAT_BUFFER.lock().unwrap().my_name = name;
                self.ignore_this_entity = Some(id);
                self.player = Some(PlayerContainer {
                    player,
                    entity_id: None,
                });
            }
            SteadyPacket::FinaliseMapLoad => {
                self.initialise_entities();
            }
            SteadyPacket::RemoveEntity(entity_id) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                let entity_index = self.get_entity_index(entity_id);
                if let Some(entity_index) = entity_index {
                    self.world.entities.remove(entity_index);
                    debug!("remove entity message received");
                    debug!("world entities: {:?}", self.world.entities);
                }
            }
            SteadyPacket::ChatMessage(who_sent, message) => {
                let mut dont_show = false;
                if let Some(player) = &self.player {
                    if player.player.uuid == who_sent {
                        dont_show = true;
                    }
                }
                if !dont_show {
                    let players = self.world.entities
                        .iter()
                        .filter(|e| e.has_component(COMPONENT_TYPE_PLAYER.clone()))
                        .collect::<Vec<&Entity>>();
                    let name = {
                        let mut namebuf = None;
                        for player in players {
                            if
                                let Some(player_component) = player.get_component(
                                    COMPONENT_TYPE_PLAYER.clone()
                                )
                            {
                                let uuid = player_component.get_parameter("uuid");
                                let uuid = match &uuid.value {
                                    ParameterValue::String(uuid) => uuid,
                                    _ => panic!("uuid is not a string"),
                                };
                                let name = player_component.get_parameter("name");
                                let name = match &name.value {
                                    ParameterValue::String(name) => name,
                                    _ => panic!("name is not a string"),
                                };

                                if uuid == &who_sent {
                                    namebuf = Some(name.clone());
                                    break;
                                }
                            }
                        }
                        namebuf
                    };
                    if let Some(name) = name {
                        chat::write_chat(name, message);
                    } else {
                        chat::write_chat(who_sent, message);
                    }
                }
            }
            SteadyPacket::SetName(who_sent, new_name) => {
                let players = self.world.entities
                    .iter_mut()
                    .filter(|e| e.has_component(COMPONENT_TYPE_PLAYER.clone()))
                    .collect::<Vec<&mut Entity>>();
                let name = {
                    let mut namebuf = None;
                    for player in players {
                        if
                            let Some(player_component) = player
                                .get_component(COMPONENT_TYPE_PLAYER.clone())
                                .cloned()
                        {
                            let uuid = player_component.get_parameter("uuid");
                            let uuid = match &uuid.value {
                                ParameterValue::String(uuid) => uuid,
                                _ => panic!("uuid is not a string"),
                            };
                            let name = player_component.get_parameter("name");
                            let name = match &name.value {
                                ParameterValue::String(name) => name,
                                _ => panic!("name is not a string"),
                            };

                            if uuid == &who_sent {
                                player.set_component_parameter(
                                    COMPONENT_TYPE_PLAYER.clone(),
                                    "name",
                                    ParameterValue::String(new_name.clone())
                                );
                                namebuf = Some(name.clone());
                                break;
                            }
                        }
                    }
                    namebuf
                };
                if let Some(name) = name {
                    chat::write_chat(
                        "server".to_string(),
                        format!("{} is now known as {}", name, new_name)
                    );
                } else {
                    chat::write_chat(
                        "server".to_string(),
                        format!("{} is now known as {}", who_sent, new_name)
                    );
                }
            }
            SteadyPacket::NameRejected(reason) =>
                match reason {
                    NameRejectionReason::IllegalWord => {
                        chat::write_chat(
                            "server".to_string(),
                            "whoa there! we don't use that kind of language here!".to_string()
                        );
                    }
                    NameRejectionReason::Taken => {
                        chat::write_chat(
                            "server".to_string(),
                            "your name was rejected because it is already taken".to_string()
                        );
                    }
                }
            SteadyPacket::ThrowThrowAballll(uuid, position, initial_velocity) => {
                let mut already_have = false;
                for tball in &self.tballs {
                    if tball.uuid == uuid {
                        already_have = true;
                        break;
                    }
                }
                if !already_have {
                    let tball = ThrowingBall::new_with_uuid(
                        uuid,
                        position,
                        initial_velocity,
                        self.physics.lock().unwrap().as_ref().unwrap()
                    );
                    self.tballs.push(tball);
                }
            }
            SteadyPacket::Respawn(position) => {
                if let Some(player) = &mut self.player {
                    info!("respawning player");
                    player.player.set_position(position);
                }
            }
            SteadyPacket::Ping => {}
        }
    }

    async fn process_steady_messages(&mut self) {
        if let Some(connection) = self.server_connection.clone() {
            match connection {
                ConnectionClientside::Local(connection) => {
                    let mut connection = connection.lock().await;

                    let try_recv = connection.steady_update_receiver.try_recv();
                    if let Ok(message) = try_recv {
                        drop(connection);
                        self.handle_steady_message(message.clone().packet).await;
                    } else if let Err(e) = try_recv {
                        if e != TryRecvError::Empty {
                            warn!("process_steady_messages: error receiving message: {:?}", e);
                        }
                    }
                }
                ConnectionClientside::Lan(connection) => {
                    let try_recv = connection.attempt_receive_steady_and_deserialise().await;
                    if let Some(message) = try_recv {
                        self.handle_steady_message(message.clone().packet).await;
                    }
                }
            }
        }
    }

    async fn handle_message_fast(&mut self, packet: FastPacket) {
        match packet.clone() {
            FastPacket::ChangePosition(entity_id, vec3) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                if let Some(entity_index) = self.get_entity_index(entity_id) {
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    let transform = entity.set_component_parameter(
                        COMPONENT_TYPE_TRANSFORM.clone(),
                        "position",
                        ParameterValue::Vec3(vec3)
                    );
                    if transform.is_none() {
                        warn!("process_fast_messages: failed to set transform rotation");
                    }
                }
            }
            FastPacket::ChangeRotation(entity_id, quat) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                if let Some(entity_index) = self.get_entity_index(entity_id) {
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    let transform = entity.set_component_parameter(
                        COMPONENT_TYPE_TRANSFORM.clone(),
                        "rotation",
                        ParameterValue::Quaternion(quat)
                    );
                    if transform.is_none() {
                        warn!("process_fast_messages: failed to set transform rotation");
                    }
                }
            }
            FastPacket::ChangeScale(entity_id, vec3) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                if let Some(entity_index) = self.get_entity_index(entity_id) {
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    let transform = entity.set_component_parameter(
                        COMPONENT_TYPE_TRANSFORM.clone(),
                        "scale",
                        ParameterValue::Vec3(vec3)
                    );
                    if transform.is_none() {
                        warn!("process_fast_messages: failed to set transform scale");
                    }
                }
            }
            FastPacket::PlayerMoved(entity_id, new_position, new_rotation, new_head_rotation) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                if let Some(entity_index) = self.get_entity_index(entity_id) {
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    let prev_transform = entity.get_component(COMPONENT_TYPE_PLAYER.clone());
                    if let Some(prev_transform) = prev_transform {
                        let prev_position = prev_transform.get_parameter("position");

                        let prev_position = match prev_position.value {
                            ParameterValue::Vec3(vec3) => vec3,
                            _ => {
                                warn!("process_fast_messages: failed to get previous position");
                                return;
                            }
                        };

                        let position_diff = new_position - prev_position;
                        let forward_mag = position_diff.dot(new_rotation.forward());
                        let strafe_mag = position_diff.dot(new_rotation.right());
                        const threshold: f32 = 0.01;
                        let forward_mag = if forward_mag.abs() < threshold {
                            0.0
                        } else {
                            1.0 * forward_mag.signum()
                        };
                        let strafe_mag = if strafe_mag.abs() < threshold {
                            0.0
                        } else {
                            1.0 * strafe_mag.signum()
                        };

                        let player_component = entity.set_component_parameter(
                            COMPONENT_TYPE_PLAYER.clone(),
                            "speed",
                            ParameterValue::Float(forward_mag as f64)
                        );
                        if player_component.is_none() {
                            warn!("process_fast_messages: failed to set transform position");
                        }
                        let player_component = entity.set_component_parameter(
                            COMPONENT_TYPE_PLAYER.clone(),
                            "strafe",
                            ParameterValue::Float(strafe_mag as f64)
                        );
                        if player_component.is_none() {
                            warn!("process_fast_messages: failed to set transform position");
                        }
                    }

                    let player_component = entity.set_component_parameter(
                        COMPONENT_TYPE_PLAYER.clone(),
                        "position",
                        ParameterValue::Vec3(new_position)
                    );
                    if player_component.is_none() {
                        warn!("process_fast_messages: failed to set transform position");
                    }
                    let player_component = entity.set_component_parameter(
                        COMPONENT_TYPE_PLAYER.clone(),
                        "rotation",
                        ParameterValue::Quaternion(new_rotation)
                    );
                    if player_component.is_none() {
                        warn!("process_fast_messages: failed to set transform rotation");
                    }
                    let player_component = entity.set_component_parameter(
                        COMPONENT_TYPE_PLAYER.clone(),
                        "head_rotation",
                        ParameterValue::Quaternion(new_head_rotation)
                    );
                    if player_component.is_none() {
                        warn!("process_fast_messages: failed to set transform rotation");
                    }
                }
            }
            FastPacket::EntitySetParameter(
                entity_id,
                component_type,
                parameter_name,
                parameter_value,
            ) => {
                if let Some(ignore) = self.ignore_this_entity {
                    if entity_id == ignore {
                        return;
                    }
                }
                if let Some(entity_index) = self.get_entity_index(entity_id) {
                    let entity = self.world.entities.get_mut(entity_index).unwrap();
                    let component = entity.set_component_parameter(
                        component_type,
                        parameter_name.as_str(),
                        parameter_value
                    );
                    if component.is_none() {
                        warn!("process_fast_messages: failed to set component parameter");
                    }
                }
            }
            FastPacket::PlayerFuckYouMoveHere(new_position) => {
                if let Some(player) = self.player.as_mut() {
                    warn!(
                        "we moved too fast, so the server is telling us to move to a new position"
                    );
                    player.player.set_position(new_position);
                }
            }
            FastPacket::PlayerFuckYouSetRotation(new_rotation) => {
                if let Some(player) = self.player.as_mut() {
                    warn!(
                        "we did something wrong, so the server is telling us to set our rotation"
                    );
                    player.player.set_rotation(new_rotation);
                    player.player.set_head_rotation(new_rotation);
                }
            }
            FastPacket::PlayerCheckPosition(_, _) => {}
            FastPacket::PlayerMove(_, _, _, _, _, _) => {}
            FastPacket::PlayerJump(_) => {}
        }
    }

    async fn process_fast_messages(&mut self) {
        if let Some(connection) = self.server_connection.clone() {
            match connection {
                ConnectionClientside::Local(connection) => {
                    let mut connection = connection.lock().await;

                    let try_recv = connection.fast_update_receiver.try_recv();
                    drop(connection);
                    if let Ok(message) = try_recv {
                        self.handle_message_fast(message.clone().packet.unwrap()).await;
                    } else if let Err(e) = try_recv {
                        if e != TryRecvError::Empty {
                            warn!("process_steady_messages: error receiving message: {:?}", e);
                        }
                    }
                }
                ConnectionClientside::Lan(connection) => {
                    let try_recv = connection.attempt_receive_fast_and_deserialise().await;
                    if let Some(message) = try_recv {
                        self.handle_message_fast(message.clone().packet.unwrap()).await;
                    }
                }
            }
        }
    }

    async fn process_client_updates(&mut self, client_updates: &mut Vec<ClientUpdate>) {
        let mut updates = Vec::new();
        let mut movement_updates = Vec::new();
        let mut jumped_real = false;
        let mut movement_info = None;
        for client_update in client_updates {
            match client_update {
                ClientUpdate::IDisplaced(displacement_vector) => {
                    let position = self.player.as_mut().unwrap().player.get_position();
                    let rotation = self.player.as_mut().unwrap().player.get_rotation();
                    let head_rotation = self.player.as_mut().unwrap().player.get_head_rotation();
                    if movement_info.is_none() {
                        if let Some(movement_info_some) = displacement_vector.1 {
                            movement_info = Some(movement_info_some);
                        }
                    }
                    movement_updates.push(
                        ClientUpdate::IMoved(
                            position,
                            Some(displacement_vector.0),
                            rotation,
                            head_rotation,
                            movement_info
                        )
                    );
                }
                ClientUpdate::ILooked(look_quat) => {
                    let position = self.player.as_mut().unwrap().player.get_position();
                    let rotation = self.player.as_mut().unwrap().player.get_rotation();
                    let head_rotation = self.player.as_mut().unwrap().player.get_head_rotation();
                    movement_updates.push(
                        ClientUpdate::IMoved(position, None, rotation, head_rotation, movement_info)
                    );
                }
                ClientUpdate::IJumped => {
                    jumped_real = true;
                }
                _ => {
                    updates.push(client_update.clone());
                }
            }
        }

        let mut last_displacement_vector = None;
        if movement_updates.len() > 0 {
            for update in movement_updates.clone() {
                if let ClientUpdate::IMoved(_, displacement_vector, _, _, _) = update {
                    last_displacement_vector = displacement_vector;
                }
            }
            let mut latest_movement_update = movement_updates.last().unwrap().clone();

            if let Some(displacement_vector) = last_displacement_vector {
                if
                    let ClientUpdate::IMoved(position, _, rotation, head_rotation, jumped) =
                        latest_movement_update
                {
                    let new = ClientUpdate::IMoved(
                        position,
                        Some(displacement_vector),
                        rotation,
                        head_rotation,
                        movement_info
                    );
                    latest_movement_update = new;
                }
            }
            updates.push(latest_movement_update.clone());
        }

        for update in updates {
            match update {
                ClientUpdate::IDisplaced(_) => {}
                ClientUpdate::ILooked(_) => {}
                ClientUpdate::IMoved(
                    position,
                    displacement_vector,
                    rotation,
                    head_rotation,
                    jumped,
                ) => {
                    let uuid = self.player.as_ref().unwrap().player.uuid.clone();
                    let displacement_vector = displacement_vector.unwrap_or(
                        Vec3::new(0.0, 0.0, 0.0)
                    );
                    let packet = FastPacket::PlayerMove(
                        uuid,
                        position,
                        displacement_vector,
                        rotation,
                        head_rotation,
                        movement_info
                    );
                    self.send_fast_message(FastPacketData {
                        packet: Some(packet),
                    }).await;
                }
                ClientUpdate::IJumped => {
                    let uuid = self.player.as_ref().unwrap().player.uuid.clone();
                    let packet = FastPacket::PlayerJump(uuid);
                    self.send_fast_message(FastPacketData {
                        packet: Some(packet),
                    }).await;
                }
                ClientUpdate::IThrewtball => {
                    self.throw_tball().await;
                }
            }
        }
    }

    async fn ping_if_needed(&mut self) {
        if self.last_ping.elapsed().as_secs_f32() > 5.0 {
            let res = self.send_steady_message(SteadyPacketData {
                packet: SteadyPacket::Ping,
                uuid: server::generate_uuid(),
            }).await;
            if !res {
                crate::ui::DISCONNECTED.store(true, Ordering::Relaxed);
            }
            self.last_ping = Instant::now();
        }
    }

    pub async fn tick_connection(&mut self, client_updates: &mut Vec<ClientUpdate>) {
        self.process_steady_messages().await;
        self.process_fast_messages().await;
        self.process_client_updates(client_updates).await;
        self.ping_if_needed().await;
    }

    pub async fn server_tick(&mut self) -> Option<Vec<WorldUpdate>> {
        let mut updates = Vec::new();

        let mut world_updates = self.world_update_queue.lock().await;
        world_updates.drain(..).for_each(|update| {
            updates.push(update);
        });
        drop(world_updates);

        if !updates.is_empty() {
            Some(updates)
        } else {
            None
        }
    }

    pub async fn queue_update(&mut self, update: WorldUpdate) {
        if !self.is_server {
            warn!("queue_update: called on client");
        } else {
            let mut world_updates = self.world_update_queue.lock().await;
            world_updates.push_back(update);
        }
    }

    pub fn queue_updates(&mut self, updates: Vec<WorldUpdate>) {
        if !self.is_server {
            warn!("queue_update: called on client");
        } else {
            let world_updates = self.world_update_queue.clone();
            tokio::spawn(async move {
                let mut world_updates = world_updates.lock().await;
                updates.iter().for_each(|update| {
                    world_updates.push_back(update.clone());
                });
            });
        }
    }

    pub async fn client_tick(
        &mut self,
        renderer: &mut MutRenderer,
        physics_engine: PhysicsSystem,
        delta_time: f32
    ) -> Vec<ClientUpdate> {
        if self.is_server {
            warn!("client_tick: called on server");
            return vec![];
        }

        let mut updates = Vec::new();

        if let Some(player_container) = self.player.as_mut() {
            let player = &mut player_container.player;
            let player_updates = player.handle_input(renderer, delta_time);
            if let Some(mut player_updates) = player_updates {
                updates.append(&mut player_updates);
            }
        }

        let mut tballs_to_remove = Vec::new();
        for (i, tball) in self.tballs.iter_mut().enumerate() {
            tball.time_to_live -= delta_time;
            if tball.time_to_live <= 0.0 {
                tballs_to_remove.push(i);
            }
        }

        for (i, tball) in tballs_to_remove.iter().enumerate() {
            self.tballs.remove(*tball - i);
        }

        if self.last_ping.elapsed().as_secs_f32() >= 10.0 {
            crate::ui::UNSTABLE_CONNECTION.store(true, Ordering::Relaxed);
        } else {
            crate::ui::UNSTABLE_CONNECTION.store(false, Ordering::Relaxed);
        }

        if self.last_ping.elapsed().as_secs_f32() >= 30.0 {
            crate::ui::DISCONNECTED.store(true, Ordering::Relaxed);
        }

        updates
    }

    pub fn next_frame(&mut self, renderer: &mut MutRenderer) {
        for mesh in &mut renderer.meshes.values_mut() {
            mesh.updated_animations_this_frame = false;
            if let Some(shadow_mesh) = &mesh.shadow_mesh {
                shadow_mesh.lock().unwrap().updated_animations_this_frame = false;
            }
        }
    }

    pub fn render(&mut self, renderer: &mut MutRenderer, shadow_pass: Option<(u8, usize)>) {
        if let Some(player) = &mut self.player {
            let position = player.player.get_position();
            let rotation = player.player.get_rotation();
            if let Some(mut mesh) = renderer.meshes.get("player").cloned() {
                renderer.meshes.get_mut("player").unwrap().updated_animations_this_frame = false;
                if let Some(shadow_mesh) = &renderer.meshes.get_mut("player").unwrap().shadow_mesh {
                    shadow_mesh.lock().unwrap().updated_animations_this_frame = false;
                }
                let texture = renderer.textures.get("default").cloned().unwrap();
                mesh.position = position + rotation.forward() * -0.2 + Vec3::new(0.0, -0.1, 0.0);
                mesh.rotation = rotation;
                mesh.scale = Vec3::new(1.0, 1.0, 1.0);

                let move_anim = MoveAnim::from_values(player.player.speed, player.player.strafe);

                mesh.render(renderer, Some(&texture), Some(move_anim.weights()), shadow_pass);
            }
        }

        for tball in &mut self.tballs {
            let position = tball.get_position();
            if let Some(mut mesh) = renderer.meshes.get("snowball").cloned() {
                renderer.meshes.get_mut("snowball").unwrap().updated_animations_this_frame = false;
                if
                    let Some(shadow_mesh) = &renderer.meshes
                        .get_mut("snowball")
                        .unwrap().shadow_mesh
                {
                    shadow_mesh.lock().unwrap().updated_animations_this_frame = false;
                }
                let texture = renderer.textures.get("snowball").cloned().unwrap();
                mesh.position = position;
                mesh.rotation = Quaternion::default();
                mesh.scale = Vec3::new(0.5, 0.5, 0.5);

                mesh.render(renderer, Some(&texture), None, shadow_pass);
            }
        }

        let lights = self.send_lights_to_renderer();
        if let Some(..) = lights {
            renderer.set_lights(lights.unwrap());
        }
        let mut indices_to_remove = Vec::new();
        for index in self.entities_wanting_to_load_things.clone() {
            let entity = &self.world.entities[index];
            let components = entity.get_components();
            let mut finished_loading = components.len();
            for component in components {
                match component.get_type() {
                    x if x == COMPONENT_TYPE_MESH_RENDERER.clone() => {
                        let mesh = component.get_parameter("mesh");
                        let mesh = match &mesh.value {
                            ParameterValue::String(v) => Some(v),
                            _ => {
                                error!("render: mesh is not a string");
                                None
                            }
                        };
                        let mesh = mesh.unwrap();
                        let texture = component.get_parameter("texture");
                        let texture = match &texture.value {
                            ParameterValue::String(v) => Some(v),
                            _ => {
                                error!("render: texture is not a string");
                                None
                            }
                        };
                        let texture = texture.unwrap();
                        let res = renderer.load_mesh_if_not_loaded(mesh);
                        if res.is_err() {
                            warn!("render: failed to load mesh '{}': {:?}", mesh, res);
                        }
                        let mesh_loaded = res.unwrap();
                        let res = renderer.load_texture_if_not_loaded(texture);
                        if res.is_err() {
                            warn!("render: failed to load texture '{}': {:?}", texture, res);
                        }
                        let texture_loaded = res.unwrap();
                        if mesh_loaded && texture_loaded {
                            finished_loading -= 1;
                        }
                    }
                    x if x == COMPONENT_TYPE_TERRAIN.clone() => {
                        let name = component.get_parameter("name");
                        let name = match &name.value {
                            ParameterValue::String(v) => Some(v),
                            _ => {
                                error!("render: terrain name is not a string");
                                None
                            }
                        };
                        let name = name.unwrap();

                        let terrain_loaded = true;
                        if terrain_loaded {
                            finished_loading -= 1;
                        }
                    }
                    x if x == COMPONENT_TYPE_LIGHT.clone() => {
                        self.lights_changed = true;
                        finished_loading -= 1;
                    }
                    _ => {
                        finished_loading -= 1;
                    }
                }
            }
            if finished_loading == 0 {
                indices_to_remove.push(index);
            }
        }
        self.entities_wanting_to_load_things.retain(|x| !indices_to_remove.contains(x));
        for (i, entity) in self.world.entities.iter_mut().enumerate() {
            if self.entities_wanting_to_load_things.contains(&i) {
                continue;
            }
            if let Some(mesh_renderer) = entity.get_component(COMPONENT_TYPE_MESH_RENDERER.clone()) {
                let mesh_name = match mesh_renderer.get_parameter("mesh").value {
                    ParameterValue::String(ref s) => s.clone(),
                    _ => {
                        error!("render: mesh is not a string");
                        continue;
                    }
                };

                if mesh_name == "Plane" {
                    if let Some((pass, _)) = shadow_pass {
                        if pass == 1 {
                            continue;
                        }
                    }
                }

                let mesh = renderer.meshes.get(&*mesh_name).cloned();
                if let Some(mut mesh) = mesh {
                    let casts_shadow = mesh_renderer.get_parameter("casts_shadow");
                    let casts_shadow = match casts_shadow.value {
                        ParameterValue::Bool(v) => v,
                        _ => {
                            error!("render: casts_shadow is not a bool");
                            true
                        }
                    };

                    if !casts_shadow {
                        if let Some((pass, _)) = shadow_pass {
                            if pass == 1 {
                                continue;
                            }
                        }
                    }
                    let texture = mesh_renderer.get_parameter("texture");
                    let texture_name = match texture.value {
                        ParameterValue::String(ref s) => s.clone(),
                        _ => {
                            error!("render: texture is not a string");
                            continue;
                        }
                    };
                    let texture = renderer.textures.get(&*texture_name).cloned();
                    if texture.is_none() {
                        error!("texture not found: {:?}", texture_name);
                        continue;
                    }
                    let texture = texture.unwrap();

                    let old_position = mesh.position;
                    let old_rotation = mesh.rotation;
                    let old_scale = mesh.scale;

                    if let Some(transform) = entity.get_component(COMPONENT_TYPE_TRANSFORM.clone()) {
                        let position = match transform.get_parameter("position").value {
                            ParameterValue::Vec3(v) => v,
                            _ => {
                                error!("render: transform position is not a vec3");
                                continue;
                            }
                        };
                        mesh.position += position;
                        let rotation = match transform.get_parameter("rotation").value {
                            ParameterValue::Quaternion(v) => v,
                            _ => {
                                error!("render: transform rotation is not a quaternion");
                                continue;
                            }
                        };

                        mesh.rotation = rotation;
                        let scale = match transform.get_parameter("scale").value {
                            ParameterValue::Vec3(v) => v,
                            _ => {
                                error!("render: transform scale is not a vec3");
                                continue;
                            }
                        };
                        mesh.scale *= scale;
                    }

                    let mut anim_weights = None;
                    if mesh_name == "player" {
                        let move_anim = MoveAnim::from_values(0.0, 0.0);
                        anim_weights = Some(move_anim.weights());
                    }

                    mesh.render(renderer, Some(&texture), anim_weights, shadow_pass);
                    mesh.position = old_position;
                    mesh.rotation = old_rotation;
                    mesh.scale = old_scale;
                    *renderer.meshes.get_mut(&*mesh_name).unwrap() = mesh;
                } else {
                    self.entities_wanting_to_load_things.push(i);
                }
            }

            if let Some(player_component) = entity.get_component(COMPONENT_TYPE_PLAYER.clone()) {
                if let Some(ignore) = self.ignore_this_entity {
                    if ignore == entity.uid {
                        continue;
                    }
                }
                let position = player_component.get_parameter("position");
                let position = match position.value {
                    ParameterValue::Vec3(v) => v,
                    _ => {
                        error!("render: player position is not a vec3");
                        continue;
                    }
                };
                let rotation = player_component.get_parameter("rotation");
                let rotation = match rotation.value {
                    ParameterValue::Quaternion(v) => v,
                    _ => {
                        error!("render: player rotation is not a quaternion");
                        continue;
                    }
                };
                let speed = player_component.get_parameter("speed");
                let speed = match speed.value {
                    ParameterValue::Float(v) => v,
                    _ => {
                        error!("render: player speed is not a float");
                        continue;
                    }
                };
                let strafe = player_component.get_parameter("strafe");
                let strafe = match strafe.value {
                    ParameterValue::Float(v) => v,
                    _ => {
                        error!("render: player strafe is not a float");
                        continue;
                    }
                };
                if let Some(mesh) = renderer.meshes.get("player").cloned() {
                    renderer.meshes
                        .get_mut("player")
                        .unwrap().updated_animations_this_frame = false;
                    if
                        let Some(shadow_mesh) = &renderer.meshes
                            .get_mut("player")
                            .unwrap().shadow_mesh
                    {
                        shadow_mesh.lock().unwrap().updated_animations_this_frame = false;
                    }
                    let texture = renderer.textures.get("default").cloned().unwrap();
                    let mut mesh = mesh.clone();
                    let old_position = mesh.position;
                    let old_rotation = mesh.rotation;
                    mesh.position = position + Vec3::new(0.0, -0.1, 0.0);
                    mesh.rotation = rotation;
                    mesh.scale = Vec3::new(1.0, 1.0, 1.0);

                    let move_anim = MoveAnim::from_values(speed, strafe);

                    mesh.render(renderer, Some(&texture), Some(move_anim.weights()), shadow_pass);

                    mesh.position = old_position;
                    mesh.rotation = old_rotation;
                    *renderer.meshes.get_mut("player").unwrap() = mesh;
                }
            }
        }
    }

    pub fn handle_audio(
        &mut self,
        renderer: &MutRenderer,
        audio: &AudioBackend,
        scontext: &SoundContext
    ) {
        audio.update(
            renderer.camera.get_position(),
            -renderer.camera.get_front(),
            renderer.camera.get_up(),
            scontext
        );

        for index in self.entities_wanting_to_load_things.clone() {
            let entity = &self.world.entities[index];
            let components = entity.get_components();
            for component in components {
                match component.get_type() {
                    x if x == COMPONENT_TYPE_JUKEBOX.clone() => {
                        let track = component.get_parameter("track");
                        let track = match track.value {
                            ParameterValue::String(ref s) => s.clone(),
                            _ => {
                                error!("audio: jukebox track is not a string");
                                continue;
                            }
                        };

                        if !audio.is_sound_loaded(&track) {
                            audio.load_sound(&track);
                        }
                    }
                    _ => {}
                }
            }
        }

        for (i, entity) in self.world.entities.iter_mut().enumerate() {
            if let Some(jukebox) = entity.get_component(COMPONENT_TYPE_JUKEBOX.clone()) {
                let track = jukebox.get_parameter("track");
                let track = match track.value {
                    ParameterValue::String(ref s) => s.clone(),
                    _ => {
                        error!("audio: jukebox track is not a string");
                        continue;
                    }
                };
                let volume = jukebox.get_parameter("volume");
                let volume = match volume.value {
                    ParameterValue::Float(v) => v,
                    _ => {
                        error!("audio: jukebox volume is not a float");
                        continue;
                    }
                };
                let playing = jukebox.get_parameter("playing");
                let playing = match playing.value {
                    ParameterValue::Bool(ref s) => s.clone(),
                    _ => {
                        error!("audio: jukebox playing is not a string");
                        continue;
                    }
                };
                let uuid = jukebox.get_parameter("uuid");
                let uuid = match uuid.value {
                    ParameterValue::String(ref s) => s.clone(),
                    _ => {
                        error!("audio: jukebox uuid is not a string");
                        continue;
                    }
                };

                let position = if
                    let Some(transform) = entity.get_component(COMPONENT_TYPE_TRANSFORM.clone())
                {
                    let position = transform.get_parameter("position");
                    let position = match position.value {
                        ParameterValue::Vec3(v) => v,
                        _ => {
                            error!("audio: transform position is not a vec3");
                            continue;
                        }
                    };
                    position
                } else {
                    Vec3::new(0.0, 0.0, 0.0)
                };

                if audio.is_sound_loaded(&track) {
                    if playing && !audio.is_sound_playing(&uuid) {
                        audio.play_sound_with_uuid(&uuid, &track, scontext);
                    } else if !playing && audio.is_sound_playing(&uuid) {
                        audio.stop_sound_with_uuid(&uuid, scontext);
                    }
                    if playing {
                        audio.set_sound_position(&uuid, position, scontext);
                    }
                } else {
                    self.entities_wanting_to_load_things.push(i);
                }
            }
        }
    }
}
