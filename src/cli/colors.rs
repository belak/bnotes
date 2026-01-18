use std::io::IsTerminal;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream};

/// Create a StandardStream with appropriate color support
///
/// Follows the termcolor recommended pattern:
/// - Respects user preference from --color flag
/// - When Auto, checks IsTerminal to disable for pipes/redirects
/// - ColorChoice::Auto also respects NO_COLOR environment variable
pub fn create_stdout(preference: ColorChoice) -> StandardStream {
    let choice = if preference == ColorChoice::Auto && !std::io::stdout().is_terminal() {
        ColorChoice::Never
    } else {
        preference
    };
    StandardStream::stdout(choice)
}

/// Error color: red + bold
pub fn error() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Red)).set_bold(true);
    spec
}

/// Warning color: yellow + bold
pub fn warning() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Yellow)).set_bold(true);
    spec
}

/// Success color: green
pub fn success() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Green));
    spec
}

/// Highlight color: cyan (for structure)
pub fn highlight() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Cyan));
    spec
}

/// Dim color: gray (for secondary info)
pub fn dim() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_dimmed(true);
    spec
}
