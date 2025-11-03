pub use insta::{assert_yaml_snapshot, with_settings};
use tracing::Level;

#[macro_export]
macro_rules! insta_test {
    ($f: expr) => {
        #[cfg(not(target_os = "windows"))]
        $crate::with_settings!({filters => vec![]}, {
            ($f)()
        })
    };
}

pub fn logging() {
    use std::sync::Once;

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .init();
    });
}
