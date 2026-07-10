mod google_live;
mod openai_realtime;

pub use google_live::test_live_translation_connection as test_google_live_translation_connection;
pub use openai_realtime::{run_realtime_translation, test_realtime_connection, RealtimeControl};
