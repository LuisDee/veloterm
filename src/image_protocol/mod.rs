// Kitty Graphics Protocol implementation.
//
// Supports inline image display via APC sequences:
//   ESC_G <control_data> ; <payload> ESC\
//
// Modules:
// - kitty_parser: Parse control key=value pairs into typed commands
// - image_store: Image data + placement management with memory limits
// - decode: PNG/RGB/RGBA decoding, zlib decompression, security validation
// - response: APC response formatting for PTY write-back

pub mod decode;
pub mod image_store;
pub mod kitty_parser;
pub mod response;

pub use image_store::{ImageData, ImageStore, Placement};
pub use kitty_parser::{parse_command, parse_control_data, Action, KittyCommand};
pub use response::{format_response, format_response_with_quiet, ResponseKind};
