mod amplify_ctrl;
mod play_pause_ctrl;
mod fade_out_ctrl;
mod wait;

pub use self::amplify_ctrl::{amplify_ctrl, AmplifyCtrl};
pub use self::play_pause_ctrl::{play_pause_ctrl, PlayPauseCtrl};
pub use self::fade_out_ctrl::{fade_out_ctrl, FadeOutCtrl};
pub use self::wait::{wait, Wait};
