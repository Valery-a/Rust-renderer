use std::cell::UnsafeCell;
use std::ffi::c_void;
use halfbrown::HashMap;
use std::ptr::{ null_mut };
use std::sync::{ Arc };
use mutex_timeouts::std::MutexWithTimeout as Mutex;
use std::sync::atomic::{ AtomicUsize, Ordering };
use gfx_maths::{ Vec3 };
use physx_sys::*;

lazy_static! {
    static ref BOX_COLLIDERS: Arc<Mutex<Vec<PhysicsBoxColliderStatic>>> = Arc::new(
        Mutex::new(Vec::new())
    );
    static ref TRIGGER_SHAPES: Arc<Mutex<Vec<PhysicsTriggerShape>>> = Arc::new(
        Mutex::new(Vec::new())
    );
    static ref PHYSICS_SYSTEM: Arc<Mutex<Option<PhysicsSystem>>> = Arc::new(Mutex::new(None));

    pub static ref PHYSICS_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub const GRAVITY: f32 = -9.81;
pub const PLAYER_GRAVITY: f32 = -24.36;
pub const PLAYER_TERMINAL_VELOCITY: f32 = 90.0;
pub const PLAYER_JUMP_VELOCITY: f32 = 12.3;

#[derive(Clone)]
pub struct PhysicsSystem {
    pub foundation: Arc<Mutex<*mut PxFoundation>>,
    pub physics: Arc<Mutex<*mut PxPhysics>>,
    pub dispatcher: Arc<Mutex<*mut PxDefaultCpuDispatcher>>,
    pub scene: *mut PxScene,
    pub controller_manager: *mut PxControllerManager,
    pub physics_materials: HashMap<Materials, PhysicsMaterial>,
}

unsafe impl Send for PhysicsSystem {}

unsafe impl Sync for PhysicsSystem {}

unsafe extern "C" fn on_trigger(_: *mut c_void, b: *const PxTriggerPair, n_pairs: u32) {
    let pairs = std::slice::from_raw_parts(b, n_pairs as usize);
    debug!("trigger pairs: {}", pairs.len());
}

impl PhysicsSystem {
    pub fn init() -> Self {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let foundation = unsafe { physx_create_foundation() };
        let physics = unsafe { physx_create_physics(foundation) };
        let mut scene_desc = unsafe { PxSceneDesc_new(PxPhysics_getTolerancesScale(physics)) };
        scene_desc.gravity = PxVec3 {
            x: 0.0,
            y: GRAVITY,
            z: 0.0,
        };
        let info = SimulationEventCallbackInfo {
            trigger_callback: Some(on_trigger),
            ..Default::default()
        };
        let callbacks = unsafe { create_simulation_event_callbacks(&info) };
        scene_desc.simulationEventCallback = callbacks;

        unsafe {
            scene_desc.filterShader = get_default_simulation_filter_shader();
        }

        let nproc = num_cpus::get();

        let dispatcher = unsafe {
            phys_PxDefaultCpuDispatcherCreate(
                (nproc / 2).min(1) as u32,
                null_mut(),
                PxDefaultCpuDispatcherWaitForWorkMode::WaitForWork,
                0
            )
        };

        scene_desc.cpuDispatcher = dispatcher as *mut _;

        let scene = unsafe { PxPhysics_createScene_mut(physics, &scene_desc) };

        let controller_manager = unsafe { phys_PxCreateControllerManager(scene, true) };

        unsafe {
            PxControllerManager_setOverlapRecoveryModule_mut(controller_manager, true);
        }

        drop(lock);
        let physics_materials = Self::init_materials(physics);

        let sys = Self {
            foundation: Arc::new(Mutex::new(foundation)),
            physics: Arc::new(Mutex::new(physics)),
            dispatcher: Arc::new(Mutex::new(dispatcher)),
            scene,
            controller_manager,
            physics_materials,
        };

        PHYSICS_SYSTEM.lock().unwrap().replace(sys.clone());
        sys
    }

    pub fn cleanup() {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let mut box_colliders = BOX_COLLIDERS.lock().unwrap();
        let mut trigger_shapes = TRIGGER_SHAPES.lock().unwrap();
        box_colliders.clear();
        trigger_shapes.clear();
        drop(lock);
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn init_materials(physics: *mut PxPhysics) -> HashMap<Materials, PhysicsMaterial> {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let mut physics_materials = HashMap::new();

        let player_material = unsafe { PxPhysics_createMaterial_mut(physics, 0.0, 0.0, 0.0) };

        physics_materials.insert(Materials::Player, PhysicsMaterial { material: player_material });

        drop(lock);
        physics_materials
    }

    pub fn copy_with_new_scene(&self) -> Self {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let mut scene_desc = unsafe {
            PxSceneDesc_new(PxPhysics_getTolerancesScale(*self.physics.lock().unwrap()))
        };
        scene_desc.gravity = PxVec3 {
            x: 0.0,
            y: GRAVITY,
            z: 0.0,
        };
        let info = SimulationEventCallbackInfo {
            trigger_callback: Some(on_trigger),
            ..Default::default()
        };
        let callbacks = unsafe { create_simulation_event_callbacks(&info) };
        scene_desc.simulationEventCallback = callbacks;

        unsafe {
            scene_desc.filterShader = get_default_simulation_filter_shader();
        }

        let dispatcher = unsafe {
            phys_PxDefaultCpuDispatcherCreate(
                2,
                null_mut(),
                PxDefaultCpuDispatcherWaitForWorkMode::WaitForWork,
                0
            )
        };

        scene_desc.cpuDispatcher = dispatcher as *mut _;

        let scene = unsafe {
            PxPhysics_createScene_mut(*self.physics.lock().unwrap(), &scene_desc)
        };

        let controller_manager = unsafe { phys_PxCreateControllerManager(scene, true) };

        unsafe {
            PxControllerManager_setOverlapRecoveryModule_mut(controller_manager, true);
        }

        drop(lock);

        Self {
            foundation: self.foundation.clone(),
            physics: self.physics.clone(),
            dispatcher: self.dispatcher.clone(),
            scene,
            controller_manager,
            physics_materials: self.physics_materials.clone(),
        }
    }

    pub fn tick(&self, delta_time: f32) -> Option<f32> {
        if delta_time <= 0.001 {
            return Some(delta_time);
        }

        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_simulate_mut(self.scene, delta_time, null_mut(), null_mut(), 0, true);
        }
        let mut error = 0u32;
        unsafe {
            PxScene_fetchResults_mut(self.scene, true, &mut error);
        }
        assert_eq!(error, 0, "physx error: {}", error);
        drop(lock);
        None
    }

    pub fn create_character_controller(
        &self,
        radius: f32,
        height: f32,
        step_offset: f32,
        material: Materials
    ) -> Option<PhysicsCharacterController> {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let mut controller_desc = unsafe { PxCapsuleControllerDesc_new_alloc() };
        unsafe {
            PxCapsuleControllerDesc_setToDefault_mut(controller_desc);
        }
        let material = self.physics_materials.get(&material).unwrap();
        unsafe {
            (*controller_desc).height = height;
            (*controller_desc).radius = radius;
            (*controller_desc).stepOffset = step_offset;
            (*controller_desc).material = material.material;

            if PxCapsuleControllerDesc_isValid(controller_desc) {
                let controller = PxControllerManager_createController_mut(
                    self.controller_manager,
                    controller_desc as *mut _
                );

                drop(lock);
                Some(PhysicsCharacterController {
                    controller: Arc::new(Mutex::new(controller)),
                    flags: Arc::new(Mutex::new(CollisionFlags::default())),
                    y_velocity: Arc::new(UnsafeCell::new(0.0)),
                })
            } else {
                drop(lock);
                None
            }
        }
    }

    pub fn create_box_collider_static(
        &self,
        position: Vec3,
        size: Vec3,
        material: Materials
    ) -> Option<PhysicsBoxColliderStatic> {
        let lock = PHYSICS_LOCK.lock().unwrap();

        let position = position;
        let size = size;

        let transform = PxTransform {
            p: PxVec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            q: PxQuat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };

        let geometry = unsafe { PxBoxGeometry_new(size.x / 2.0, size.y / 2.0, size.z / 2.0) };

        let material = self.physics_materials.get(&material).unwrap();

        let box_actor = unsafe {
            PxPhysics_createRigidStatic_mut(*self.physics.lock().unwrap(), &transform)
        };
        let shape_flags =
            (PxShapeFlag::SimulationShape as u8) | (PxShapeFlag::SceneQueryShape as u8);
        let shape_flags = PxShapeFlags::from_bits(shape_flags).unwrap();
        let box_shape = unsafe {
            PxPhysics_createShape_mut(
                *self.physics.lock().unwrap(),
                &geometry as *const PxBoxGeometry as *const PxGeometry,
                material.material,
                true,
                shape_flags
            )
        };

        unsafe {
            PxRigidActor_attachShape_mut(box_actor as *mut PxRigidActor, box_shape);
        }

        drop(lock);
        Some(PhysicsBoxColliderStatic {
            actor: box_actor,
            shape: box_shape,
            ref_count: Arc::new(Default::default()),
        })
    }

    pub fn create_sphere_actor(
        &self,
        position: Vec3,
        radius: f32,
        material: Materials
    ) -> Option<PhysicsSphereColliderDynamic> {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = position;
        let radius = radius;

        let transform = PxTransform {
            p: PxVec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            q: PxQuat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };

        let geometry = unsafe { PxSphereGeometry_new(radius * 2.0) };

        let material = self.physics_materials.get(&material).unwrap();

        let actor = unsafe {
            phys_PxCreateDynamic(
                *self.physics.lock().unwrap(),
                &transform,
                &geometry as *const PxSphereGeometry as *const PxGeometry,
                material.material,
                1.0,
                &PxTransform_new_2(PxIDENTITY::PxIdentity)
            )
        };

        unsafe {
            PxRigidBody_setAngularDamping_mut(actor as *mut PxRigidBody, 0.5);
        }

        drop(lock);
        Some(PhysicsSphereColliderDynamic {
            actor,
            ref_count: Arc::new(Default::default()),
        })
    }

    pub fn create_trigger_shape(
        &self,
        position: Vec3,
        size: Vec3,
        material: Materials
    ) -> Option<PhysicsTriggerShape> {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = position + Vec3::new(size.x / 2.0, size.y / 2.0, -size.z / 2.0);
        let size = size;

        let transform = PxTransform {
            p: PxVec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            q: PxQuat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };

        let geometry = unsafe { PxBoxGeometry_new(size.x / 2.0, size.y / 2.0, size.z / 2.0) };

        let material = self.physics_materials.get(&material).unwrap();

        let box_actor = unsafe {
            PxPhysics_createRigidStatic_mut(*self.physics.lock().unwrap(), &transform)
        };
        let shape_flags = PxShapeFlag::TriggerShape as u8;
        let shape_flags = PxShapeFlags::from_bits(shape_flags).unwrap();
        let box_shape = unsafe {
            PxPhysics_createShape_mut(
                *self.physics.lock().unwrap(),
                &geometry as *const PxBoxGeometry as *const PxGeometry,
                material.material,
                true,
                shape_flags
            )
        };

        unsafe {
            PxRigidActor_attachShape_mut(box_actor as *mut PxRigidActor, box_shape);
        }

        drop(lock);
        Some(PhysicsTriggerShape {
            actor: box_actor,
            shape: box_shape,
            ref_count: Arc::new(Default::default()),
        })
    }
}

#[derive(Clone, Debug)]
pub enum ClimbingMode {
    Easy,
    Constrained,
    Last,
}

#[derive(Clone, Debug, Default)]
pub struct CollisionFlags {
    pub colliding_side: bool,
    pub colliding_top: bool,
    pub colliding_bottom: bool,
}

impl CollisionFlags {
    pub fn from_bits(bits: u8) -> Self {
        Self {
            colliding_side: (bits & 1) != 0,
            colliding_top: (bits & 2) != 0,
            colliding_bottom: (bits & 4) != 0,
        }
    }
}

#[derive(Clone)]
pub struct PhysicsCharacterController {
    pub controller: Arc<Mutex<*mut PxController>>,
    pub flags: Arc<Mutex<CollisionFlags>>,
    y_velocity: Arc<UnsafeCell<f32>>,
}

unsafe impl Send for PhysicsCharacterController {}

unsafe impl Sync for PhysicsCharacterController {}

impl PhysicsCharacterController {
    pub fn move_by(
        &mut self,
        displacement: Vec3,
        jump: bool,
        server: Option<bool>,
        cheat: bool,
        delta_time: f32,
        frame_delta: f32
    ) -> Vec3 {
        let lock = PHYSICS_LOCK.lock().unwrap();
        if delta_time <= 0.0 || frame_delta <= 0.0 {
            return Vec3::zero();
        }

        let mut displacement = PxVec3 {
            x: displacement.x,
            y: displacement.y,
            z: displacement.z,
        };

        let do_gravity = if let Some(server) = server { server } else { true };

        if jump && self.is_on_ground() {
            unsafe {
                *self.y_velocity.get() = PLAYER_JUMP_VELOCITY;
            }
        } else if !self.is_on_ground() {
            if do_gravity {
                let gravity = PLAYER_GRAVITY;
                let mut velocity = unsafe { *self.y_velocity.get() };
                velocity += gravity * delta_time;
                velocity = velocity.clamp(-PLAYER_TERMINAL_VELOCITY, PLAYER_TERMINAL_VELOCITY);
                unsafe {
                    *self.y_velocity.get() = velocity;
                }
            }
        } else if cheat {
            unsafe {
                *self.y_velocity.get() = 100.0;
            }
        }

        if do_gravity {
            displacement.y = unsafe { *self.y_velocity.get() };
            displacement.y *= delta_time;
        }

        unsafe {
            let flags = PxController_move_mut(
                *self.controller.lock().unwrap(),
                &displacement,
                0.0,
                delta_time,
                &PxControllerFilters_new(null_mut(), null_mut(), null_mut()),
                null_mut()
            );
            *self.flags.lock().unwrap() = CollisionFlags::from_bits(flags.bits());
        }

        drop(lock);
        Vec3::new(
            displacement.x / frame_delta,
            displacement.y / delta_time,
            displacement.z / frame_delta
        )
    }

    pub fn is_on_ground(&self) -> bool {
        let flags = self.flags.lock().unwrap();
        flags.colliding_bottom
    }

    pub fn get_position(&self) -> Vec3 {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = unsafe { PxController_getPosition(*self.controller.lock().unwrap()) };
        let x = unsafe { (*position).x };
        let y = unsafe { (*position).y };
        let z = unsafe { (*position).z };
        drop(lock);
        Vec3::new(x as f32, y as f32, z as f32)
    }

    pub fn get_foot_position(&self) -> Vec3 {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = unsafe { PxController_getFootPosition(*self.controller.lock().unwrap()) };
        let x = position.x;
        let y = position.y;
        let z = position.z;
        drop(lock);
        Vec3::new(x as f32, y as f32, z as f32)
    }

    pub fn set_position(&self, position: Vec3) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = PxExtendedVec3 {
            x: position.x as f64,
            y: position.y as f64,
            z: position.z as f64,
        };
        unsafe {
            PxController_setPosition_mut(*self.controller.lock().unwrap(), &position);
        }
        drop(lock);
    }

    pub fn set_foot_position(&self, position: Vec3) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = PxExtendedVec3 {
            x: position.x as f64,
            y: position.y as f64,
            z: position.z as f64,
        };
        unsafe {
            PxController_setFootPosition_mut(*self.controller.lock().unwrap(), &position);
        }
        drop(lock);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Materials {
    Player,
}

#[derive(Copy, Clone, Debug)]
pub struct PhysicsMaterial {
    pub material: *mut PxMaterial,
}

unsafe impl Send for PhysicsMaterial {}

unsafe impl Sync for PhysicsMaterial {}

pub struct PhysicsBoxColliderStatic {
    pub actor: *mut PxRigidStatic,
    pub shape: *mut PxShape,
    ref_count: Arc<AtomicUsize>,
}

unsafe impl Send for PhysicsBoxColliderStatic {}

unsafe impl Sync for PhysicsBoxColliderStatic {}

impl PhysicsBoxColliderStatic {
    pub fn add_self_to_scene(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_addActor_mut(physics.scene, self.actor as *mut PxActor, null_mut());
        }
        BOX_COLLIDERS.lock().unwrap().push(self.clone());
        drop(lock);
    }

    pub unsafe fn remove_self(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_removeActor_mut(physics.scene, self.actor as *mut PxActor, false);
            PxRigidActor_release_mut(self.actor as *mut PxRigidActor);
        }
        drop(lock);
    }
}

impl Drop for PhysicsBoxColliderStatic {
    fn drop(&mut self) {
        let ref_count = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        if ref_count == 0 {
            unsafe {
                self.remove_self(PHYSICS_SYSTEM.lock().unwrap().as_ref().unwrap().clone());
            }
        }
    }
}

impl Clone for PhysicsBoxColliderStatic {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            actor: self.actor,
            shape: self.shape,
            ref_count: self.ref_count.clone(),
        }
    }
}

pub struct PhysicsSphereColliderDynamic {
    pub actor: *mut PxRigidDynamic,
    ref_count: Arc<AtomicUsize>,
}

unsafe impl Send for PhysicsSphereColliderDynamic {}

unsafe impl Sync for PhysicsSphereColliderDynamic {}

impl PhysicsSphereColliderDynamic {
    pub fn add_self_to_scene(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_addActor_mut(physics.scene, self.actor as *mut PxActor, null_mut());
        }
        drop(lock);
    }

    pub fn set_velocity(&self, velocity: Vec3) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let velocity = PxVec3 {
            x: velocity.x,
            y: velocity.y,
            z: velocity.z,
        };

        unsafe {
            PxRigidBody_addForce_mut(
                self.actor as *mut PxRigidBody,
                &velocity,
                PxForceMode::VelocityChange,
                true
            );
        }
        drop(lock);
    }

    pub unsafe fn remove_self(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_removeActor_mut(physics.scene, self.actor as *mut PxActor, false);
            PxRigidActor_release_mut(self.actor as *mut PxRigidActor);
        }
        drop(lock);
    }

    pub fn get_position(&self) -> Vec3 {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = unsafe { PxRigidActor_getGlobalPose(self.actor as *const PxRigidActor).p };
        let x = position.x;
        let y = position.y;
        let z = position.z;
        drop(lock);
        Vec3::new(x, y, z)
    }

    pub fn set_position(&self, position: Vec3) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        let position = PxTransform {
            p: PxVec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            q: PxQuat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };
        unsafe {
            PxRigidActor_setGlobalPose_mut(self.actor as *mut PxRigidActor, &position, true);
        }
        drop(lock);
    }
}

impl Drop for PhysicsSphereColliderDynamic {
    fn drop(&mut self) {
        let ref_count = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        if ref_count == 0 {
            unsafe {
                self.remove_self(PHYSICS_SYSTEM.lock().unwrap().as_ref().unwrap().clone());
            }
        }
    }
}

impl Clone for PhysicsSphereColliderDynamic {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            actor: self.actor,
            ref_count: self.ref_count.clone(),
        }
    }
}

pub struct PhysicsTriggerShape {
    pub actor: *mut PxRigidStatic,
    pub shape: *mut PxShape,
    ref_count: Arc<AtomicUsize>,
}

unsafe impl Send for PhysicsTriggerShape {}

unsafe impl Sync for PhysicsTriggerShape {}

impl PhysicsTriggerShape {
    pub fn add_self_to_scene(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_addActor_mut(physics.scene, self.actor as *mut PxActor, null_mut());
        }
        TRIGGER_SHAPES.lock().unwrap().push(self.clone());
        drop(lock);
    }

    pub unsafe fn remove_self(&self, physics: PhysicsSystem) {
        let lock = PHYSICS_LOCK.lock().unwrap();
        unsafe {
            PxScene_removeActor_mut(physics.scene, self.actor as *mut PxActor, false);
            PxRigidActor_release_mut(self.actor as *mut PxRigidActor);
        }
        drop(lock);
    }
}

impl Drop for PhysicsTriggerShape {
    fn drop(&mut self) {
        let ref_count = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        if ref_count == 0 {
            unsafe {
                self.remove_self(PHYSICS_SYSTEM.lock().unwrap().as_ref().unwrap().clone());
            }
        }
    }
}

impl Clone for PhysicsTriggerShape {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            actor: self.actor,
            shape: self.shape,
            ref_count: self.ref_count.clone(),
        }
    }
}