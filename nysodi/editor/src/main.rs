//! Editor with your game connected to it as a plugin.
use fyroxed_base::{fyrox::event_loop::EventLoop, Editor, StartupData, fyrox::core::log::Log};

fn main() {
    Log::set_file_name("nysodi.log");

    let event_loop = EventLoop::new().unwrap();
    let mut editor = Editor::new(
        Some(StartupData {
            working_directory: Default::default(),
            scenes: vec!["data/scene.rgs".into()],
        }),
    );
    
     // Dynamic linking with hot reloading.
    #[cfg(feature = "dylib")]
    {
        #[cfg(target_os = "windows")]
        let file_name = "game_dylib.dll";
        #[cfg(target_os = "linux")]
        let file_name = "libgame_dylib.so";
        #[cfg(target_os = "macos")]
        let file_name = "libgame_dylib.dylib";
        editor.add_dynamic_plugin(file_name, true, true).unwrap();
    }

    // Static linking.
    #[cfg(not(feature = "dylib"))]
    {
        use nysodi::Game;
        editor.add_game_plugin(Game::default());
    }
    
    editor.run(event_loop)
}
