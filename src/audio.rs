use std::sync::{Arc, Mutex};
use fyrox_sound::algebra::Vector3;
use fyrox_sound::buffer::{DataSource, SoundBufferResource};
use fyrox_sound::context::SoundContext;
use fyrox_sound::futures::executor::block_on;
use fyrox_sound::pool::Handle;
use fyrox_sound::source::{SoundSource, SoundSourceBuilder};
use fyrox_sound::source::Status::Playing;
use gfx_maths::Vec3;
use halfbrown::HashMap;

// Global static variable to store oneshot sounds
lazy_static!{
    pub static ref ONESHOTS: Arc<Mutex<Vec<(String, Vec3)>>> = Arc::new(Mutex::new(vec![]));
}

// Struct representing the audio backend
pub struct AudioBackend {
    sounds: Arc<Mutex<HashMap<String, SoundBufferResource>>>,  // Map of sound names to their buffer resources
    playing_sounds: Arc<Mutex<HashMap<String, Handle<SoundSource>>>>,  // Map of playing sounds' UUIDs to their handles
    oneshots: Arc<Mutex<Vec<String>>>,  // List of oneshot sounds' UUIDs
}

impl AudioBackend {
    // Constructor for the AudioBackend struct
    pub fn new() -> Self {
        Self {
            sounds: Arc::new(Mutex::new(HashMap::new())),
            playing_sounds: Arc::new(Mutex::new(Default::default())),
            oneshots: Arc::new(Mutex::new(vec![])),
        }
    }

    // Load a sound into the backend by name
    pub fn load_sound(&self, name: &str) {
        let mut sounds = self.sounds.lock().unwrap();
        // Get full path
        let full_path = format!("base/snd/{}", name);
        // Load sound
        let sound = SoundBufferResource::new_generic(block_on(DataSource::from_file(&full_path)).unwrap()).expect("failed to load sound");
        // Insert into hashmap
        sounds.insert(name.to_string(), sound);
    }

    // Check if a sound is loaded
    pub fn is_sound_loaded(&self, name: &str) -> bool {
        let sounds = self.sounds.lock().unwrap();
        sounds.contains_key(name)
    }

    // Check if a sound with a given UUID is currently playing
    pub fn is_sound_playing(&self, uuid: &str) -> bool {
        let playing_sounds = self.playing_sounds.lock().unwrap();
        playing_sounds.contains_key(uuid)
    }

    // Play a sound with a specific UUID
    pub fn play_sound_with_uuid(&self, uuid: &str, name: &str, context: &SoundContext) {
        let sounds = self.sounds.lock().unwrap();
        let sound = sounds.get(name).unwrap();
        let mut playing_sounds = self.playing_sounds.lock().unwrap();
        let source = SoundSourceBuilder::new()
            .with_buffer(sound.clone())
            .with_looping(true)
            .with_status(Playing)
            .build().expect("failed to build sound source");
        let handle = context.state().add_source(source);
        playing_sounds.insert(uuid.to_string(), handle);
    }

    // Play a oneshot sound with a specific UUID and position
    pub fn play_oneshot_with_uuid(&self, uuid: &str, name: &str, context: &SoundContext, position: Vec3) {
        let sounds = self.sounds.lock().unwrap();
        let sound = sounds.get(name).unwrap();
        let mut playing_sounds = self.playing_sounds.lock().unwrap();
        let mut oneshots = self.oneshots.lock().unwrap();
        let source = SoundSourceBuilder::new()
            .with_buffer(sound.clone())
            .with_looping(false)
            .with_status(Playing)
            .build().expect("failed to build sound source");
        let handle = context.state().add_source(source);
        context.state().source_mut(handle).set_position(Vector3::new(position.x, position.y, position.z));
        playing_sounds.insert(uuid.to_string(), handle);
        oneshots.push(uuid.to_string());
    }

    // Stop a sound with a specific UUID
    pub fn stop_sound_with_uuid(&self, uuid: &str, context: &SoundContext) {
        let mut playing_sounds = self.playing_sounds.lock().unwrap();
        let handle = playing_sounds.remove(uuid);
        if let Some(handle) = handle {
            context.state().remove_source(handle);
        }
    }

    // Set the position of a sound with a specific UUID
    pub fn set_sound_position(&self, uuid: &str, position: Vec3, context: &SoundContext) {
        let playing_sounds = self.playing_sounds.lock().unwrap();
        let handle = playing_sounds.get(uuid).unwrap();
        context.state().source_mut(*handle).set_position(Vector3::new(position.x, position.y, position.z));
    }

    // Update the audio backend, including oneshot sounds
    pub fn update(&self, position: Vec3, forward: Vec3, up: Vec3, context: &SoundContext) {
        context.state().listener_mut().set_position(Vector3::new(position.x, position.y, position.z));
        context.state().listener_mut().set_orientation_rh(Vector3::new(forward.x, forward.y, forward.z), Vector3::new(up.x, up.y, up.z));
        let mut oneshots = self.oneshots.lock().unwrap();
        let mut oneshots_to_remove = vec![];
        for oneshot in oneshots.iter() {
            let playing_sounds = self.playing_sounds.lock().unwrap();
            let handle = playing_sounds.get(oneshot).unwrap();
            let state = context.state();
            let source = state.source(*handle);
            if source.status() == Playing {
                continue;
            }
            oneshots_to_remove.push(oneshot.to_string());
        }
        for oneshot in oneshots_to_remove.iter() {
            oneshots.retain(|x| x != oneshot);
        }

        drop(oneshots);

        let mut oneshots = ONESHOTS.lock().unwrap();
        // Play oneshots
        for oneshot in oneshots.iter() {
            self.play_oneshot_with_uuid(&oneshot.0, &oneshot.0, context, oneshot.1);
        }
        oneshots.clear();
    }
}
