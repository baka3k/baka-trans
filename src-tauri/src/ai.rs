mod google_live;
mod local_whisper_ollama;
mod openai_realtime;

pub use google_live::run_live_translation as run_google_live_translation;
pub use google_live::test_live_translation_connection as test_google_live_translation_connection;
pub use local_whisper_ollama::{run_local_translation, LocalTranslationRuntime};
pub use openai_realtime::{run_realtime_translation, test_realtime_connection, RealtimeControl};
