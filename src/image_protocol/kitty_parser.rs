// Kitty Graphics Protocol parser.
//
// Parses APC_G control key=value pairs into typed commands.
// Reference: https://sw.kovidgoyal.net/kitty/graphics-protocol/

use std::collections::HashMap;

/// Action requested by the graphics command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Transmit,
    TransmitAndDisplay,
    Query,
    Place,
    Delete,
    Frame,
    Animate,
    Compose,
}

/// Pixel format of image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb24,
    Rgba32,
    Png,
}

/// Transmission medium.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Medium {
    Direct,
    File,
    TempFile,
    SharedMemory,
}

/// Compression method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zlib,
}

/// Delete target specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteTarget {
    AllVisible,
    AllFree,
    ById,
    ByIdFree,
    ByNumber,
    ByNumberFree,
    AtCursor,
    AtCursorFree,
    ByZIndex,
    ByZIndexFree,
    ByColumn,
    ByColumnFree,
    ByRange,
    ByRangeFree,
}

/// Parsed Kitty Graphics command.
#[derive(Debug, Clone)]
pub struct KittyCommand {
    pub action: Action,
    pub format: PixelFormat,
    pub medium: Medium,
    pub compression: Compression,
    pub image_id: Option<u32>,
    pub image_number: Option<u32>,
    pub placement_id: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub columns: Option<u32>,
    pub rows: Option<u32>,
    pub z_index: i32,
    pub more_chunks: bool,
    pub quiet: u8,
    pub cursor_movement: bool,
    pub delete_target: Option<DeleteTarget>,
    pub payload: Vec<u8>,
    /// x parameter (used for delete range start column)
    pub x: Option<u32>,
    /// y parameter (used for delete range start row)
    pub y: Option<u32>,
}

/// Parser error.
#[derive(Debug, Clone)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "kitty parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse the full raw APC_G payload (control_data;base64_data or just control_data).
pub fn parse_command(raw: &[u8]) -> Result<KittyCommand, ParseError> {
    // Split on first ';' — left is control data, right is base64 payload
    let (control_part, payload_b64) = if let Some(pos) = raw.iter().position(|&b| b == b';') {
        (&raw[..pos], &raw[pos + 1..])
    } else {
        (raw, &[] as &[u8])
    };

    let mut cmd = parse_control_data(control_part)?;

    // Decode base64 payload
    if !payload_b64.is_empty() {
        // Strip newlines from base64 (payloads can span lines)
        let cleaned: Vec<u8> = payload_b64
            .iter()
            .copied()
            .filter(|&b| b != b'\n' && b != b'\r')
            .collect();
        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cleaned) {
            Ok(decoded) => cmd.payload = decoded,
            Err(_) => {
                return Err(ParseError("invalid base64 payload".to_string()));
            }
        }
    }

    Ok(cmd)
}

/// Parse only the control key=value data (before the ';' separator).
pub fn parse_control_data(data: &[u8]) -> Result<KittyCommand, ParseError> {
    let s = std::str::from_utf8(data).map_err(|_| ParseError("invalid UTF-8".to_string()))?;

    let mut kvs: HashMap<&str, &str> = HashMap::new();
    for pair in s.split(',') {
        if let Some((k, v)) = pair.split_once('=') {
            kvs.insert(k, v);
        }
    }

    let action = match kvs.get("a").copied() {
        Some("t") => Action::Transmit,
        Some("T") => Action::TransmitAndDisplay,
        Some("q") => Action::Query,
        Some("p") => Action::Place,
        Some("d") => Action::Delete,
        Some("f") => Action::Frame,
        Some("a") => Action::Animate,
        Some("c") => Action::Compose,
        None => Action::TransmitAndDisplay, // default
        Some(other) => {
            return Err(ParseError(format!("unknown action: {}", other)));
        }
    };

    let format = match kvs.get("f").copied() {
        Some("24") => PixelFormat::Rgb24,
        Some("32") => PixelFormat::Rgba32,
        Some("100") => PixelFormat::Png,
        None => PixelFormat::Rgba32, // default
        Some(other) => {
            return Err(ParseError(format!("unsupported format: {}", other)));
        }
    };

    let medium = match kvs.get("t").copied() {
        Some("d") => Medium::Direct,
        Some("f") => Medium::File,
        Some("t") => Medium::TempFile,
        Some("s") => Medium::SharedMemory,
        None => Medium::Direct,
        Some(other) => {
            return Err(ParseError(format!("unknown medium: {}", other)));
        }
    };

    let compression = match kvs.get("o").copied() {
        Some("z") => Compression::Zlib,
        None => Compression::None,
        Some(_) => Compression::None,
    };

    let image_id = kvs.get("i").map(|v| v.parse::<u32>().unwrap_or(0));
    let image_number = kvs.get("I").map(|v| v.parse::<u32>().unwrap_or(0));
    let placement_id = kvs.get("p").map(|v| v.parse::<u32>().unwrap_or(0));

    // Reject having both i= and I= set
    if image_id.is_some() && image_number.is_some() {
        return Err(ParseError(
            "cannot specify both i= and I= (EINVAL)".to_string(),
        ));
    }

    let width = kvs.get("s").map(|v| v.parse::<u32>().unwrap_or(0));
    let height = kvs.get("v").map(|v| v.parse::<u32>().unwrap_or(0));
    let columns = kvs.get("c").map(|v| v.parse::<u32>().unwrap_or(0));
    let rows = kvs.get("r").map(|v| v.parse::<u32>().unwrap_or(0));
    let x = kvs.get("x").map(|v| v.parse::<u32>().unwrap_or(0));
    let y = kvs.get("y").map(|v| v.parse::<u32>().unwrap_or(0));

    let z_index = kvs
        .get("z")
        .map(|v| v.parse::<i32>().unwrap_or(0))
        .unwrap_or(0);

    let more_chunks = kvs.get("m").map(|v| *v == "1").unwrap_or(false);

    let quiet = kvs
        .get("q")
        .map(|v| v.parse::<u8>().unwrap_or(0))
        .unwrap_or(0);

    let cursor_movement = kvs.get("C").map(|v| *v != "1").unwrap_or(true);

    let delete_target = if action == Action::Delete {
        Some(match kvs.get("d").copied() {
            Some("a") | None => DeleteTarget::AllVisible,
            Some("A") => DeleteTarget::AllFree,
            Some("i") => DeleteTarget::ById,
            Some("I") => DeleteTarget::ByIdFree,
            Some("n") => DeleteTarget::ByNumber,
            Some("N") => DeleteTarget::ByNumberFree,
            Some("c") => DeleteTarget::AtCursor,
            Some("C") => DeleteTarget::AtCursorFree,
            Some("z") => DeleteTarget::ByZIndex,
            Some("Z") => DeleteTarget::ByZIndexFree,
            Some("p") => DeleteTarget::ByColumn,
            Some("P") => DeleteTarget::ByColumnFree,
            Some("r") | Some("R") => {
                // Validate range: x must not be > y (if both provided as row range)
                if let (Some(xv), Some(yv)) = (x, y) {
                    if xv > yv {
                        return Err(ParseError(
                            "delete range: x must not exceed y".to_string(),
                        ));
                    }
                }
                if kvs.get("d").copied() == Some("R") {
                    DeleteTarget::ByRangeFree
                } else {
                    DeleteTarget::ByRange
                }
            }
            Some(other) => {
                return Err(ParseError(format!("unknown delete target: {}", other)));
            }
        })
    } else {
        None
    };

    Ok(KittyCommand {
        action,
        format,
        medium,
        compression,
        image_id,
        image_number,
        placement_id,
        width,
        height,
        columns,
        rows,
        z_index,
        more_chunks,
        quiet,
        cursor_movement,
        delete_target,
        payload: Vec::new(),
        x,
        y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // ── Action parsing ──────────────────────────────────────────────

    #[rstest]
    #[case(b"a=t,f=24,s=1,v=1", Action::Transmit)]
    #[case(b"a=T,f=24,s=1,v=1", Action::TransmitAndDisplay)]
    #[case(b"a=q,f=24,s=1,v=1", Action::Query)]
    #[case(b"a=p,i=1", Action::Place)]
    #[case(b"a=d", Action::Delete)]
    fn parse_action(#[case] input: &[u8], #[case] expected: Action) {
        let cmd = parse_control_data(input).unwrap();
        assert_eq!(cmd.action, expected);
    }

    #[test]
    fn default_action_is_transmit_and_display() {
        let cmd = parse_control_data(b"f=24,s=1,v=1").unwrap();
        assert_eq!(cmd.action, Action::TransmitAndDisplay);
    }

    // ── Format parsing ──────────────────────────────────────────────

    #[rstest]
    #[case(b"f=24", PixelFormat::Rgb24)]
    #[case(b"f=32", PixelFormat::Rgba32)]
    #[case(b"f=100", PixelFormat::Png)]
    fn parse_format(#[case] input: &[u8], #[case] expected: PixelFormat) {
        let cmd = parse_control_data(input).unwrap();
        assert_eq!(cmd.format, expected);
    }

    #[test]
    fn default_format_is_rgba32() {
        let cmd = parse_control_data(b"a=t,s=1,v=1").unwrap();
        assert_eq!(cmd.format, PixelFormat::Rgba32);
    }

    // ── Medium parsing ──────────────────────────────────────────────

    #[rstest]
    #[case(b"t=d", Medium::Direct)]
    #[case(b"t=f", Medium::File)]
    #[case(b"t=t", Medium::TempFile)]
    #[case(b"t=s", Medium::SharedMemory)]
    fn parse_medium(#[case] input: &[u8], #[case] expected: Medium) {
        let cmd = parse_control_data(input).unwrap();
        assert_eq!(cmd.medium, expected);
    }

    // ── Numeric fields ──────────────────────────────────────────────

    #[test]
    fn parse_image_dimensions() {
        let cmd = parse_control_data(b"s=100,v=200,f=24").unwrap();
        assert_eq!(cmd.width, Some(100));
        assert_eq!(cmd.height, Some(200));
    }

    #[test]
    fn parse_display_dimensions() {
        let cmd = parse_control_data(b"a=p,i=1,c=10,r=5").unwrap();
        assert_eq!(cmd.columns, Some(10));
        assert_eq!(cmd.rows, Some(5));
    }

    #[test]
    fn parse_z_index_negative() {
        let cmd = parse_control_data(b"a=p,i=1,z=-1073741824").unwrap();
        assert_eq!(cmd.z_index, -1073741824);
    }

    #[test]
    fn parse_z_index_positive() {
        let cmd = parse_control_data(b"a=p,i=1,z=42").unwrap();
        assert_eq!(cmd.z_index, 42);
    }

    #[test]
    fn parse_image_id_and_placement_id() {
        let cmd = parse_control_data(b"i=42,p=7").unwrap();
        assert_eq!(cmd.image_id, Some(42));
        assert_eq!(cmd.placement_id, Some(7));
    }

    #[test]
    fn parse_image_number() {
        let cmd = parse_control_data(b"I=93,s=1,v=1,f=24").unwrap();
        assert_eq!(cmd.image_number, Some(93));
    }

    // ── Chunking ────────────────────────────────────────────────────

    #[test]
    fn parse_more_chunks_flag() {
        let cmd = parse_control_data(b"m=1,i=1,s=2,v=2,f=32").unwrap();
        assert!(cmd.more_chunks);
    }

    #[test]
    fn parse_final_chunk() {
        let cmd = parse_control_data(b"m=0").unwrap();
        assert!(!cmd.more_chunks);
    }

    // ── Quiet mode ──────────────────────────────────────────────────

    #[rstest]
    #[case(b"q=0", 0)]
    #[case(b"q=1", 1)]
    #[case(b"q=2", 2)]
    fn parse_quiet(#[case] input: &[u8], #[case] expected: u8) {
        let cmd = parse_control_data(input).unwrap();
        assert_eq!(cmd.quiet, expected);
    }

    // ── Cursor movement suppression ─────────────────────────────────

    #[test]
    fn parse_cursor_movement_suppressed() {
        let cmd = parse_control_data(b"a=p,i=1,C=1").unwrap();
        assert!(!cmd.cursor_movement);
    }

    // ── Delete target parsing ───────────────────────────────────────

    #[rstest]
    #[case(b"a=d,d=a", DeleteTarget::AllVisible)]
    #[case(b"a=d,d=A", DeleteTarget::AllFree)]
    #[case(b"a=d,d=i,i=3", DeleteTarget::ById)]
    #[case(b"a=d,d=I,i=3", DeleteTarget::ByIdFree)]
    #[case(b"a=d,d=c", DeleteTarget::AtCursor)]
    #[case(b"a=d,d=C", DeleteTarget::AtCursorFree)]
    #[case(b"a=d,d=z,z=5", DeleteTarget::ByZIndex)]
    #[case(b"a=d,d=R,x=3,y=11", DeleteTarget::ByRangeFree)]
    fn parse_delete_target(#[case] input: &[u8], #[case] expected: DeleteTarget) {
        let cmd = parse_control_data(input).unwrap();
        assert_eq!(cmd.delete_target, Some(expected));
    }

    // ── Payload decoding ────────────────────────────────────────────

    #[test]
    fn parse_full_command_with_payload() {
        let raw = b"i=1,s=1,v=1,f=24;YWJj"; // "abc" base64
        let cmd = parse_command(raw).unwrap();
        assert_eq!(cmd.payload, b"abc");
    }

    #[test]
    fn parse_empty_payload() {
        let raw = b"a=d,d=a";
        let cmd = parse_command(raw).unwrap();
        assert!(cmd.payload.is_empty());
    }

    #[test]
    fn parse_png_payload_decodes() {
        let raw = b"i=1,f=100;iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
        let cmd = parse_command(raw).unwrap();
        assert_eq!(cmd.format, PixelFormat::Png);
        assert!(!cmd.payload.is_empty());
        // Verify PNG magic bytes
        assert_eq!(&cmd.payload[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    // ── Error cases ─────────────────────────────────────────────────

    #[test]
    fn parse_unknown_keys_ignored() {
        let cmd = parse_control_data(b"f=24,s=10,v=20,hello=world").unwrap();
        assert_eq!(cmd.format, PixelFormat::Rgb24);
        assert_eq!(cmd.width, Some(10));
    }

    #[test]
    fn parse_overflow_u32_returns_zero() {
        let cmd = parse_control_data(b"i=99999999999999999999").unwrap();
        assert_eq!(cmd.image_id, Some(0)); // overflow → 0
    }

    #[test]
    fn parse_both_id_and_number_is_error() {
        let result = parse_control_data(b"i=3,I=1,s=1,v=1,f=24");
        assert!(result.is_err());
    }

    #[test]
    fn parse_delete_range_x_greater_than_y_is_error() {
        let result = parse_control_data(b"a=d,d=R,x=5,y=4");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unsupported_format_is_error() {
        let result = parse_control_data(b"f=99");
        assert!(result.is_err());
    }

    // ── Property-based: parser never panics on arbitrary input ──────

    mod proptest_parser {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn parser_never_panics(data in prop::collection::vec(any::<u8>(), 0..4096)) {
                let _ = parse_command(&data); // must not panic
            }
        }

        proptest! {
            #[test]
            fn valid_control_keys_roundtrip(
                id in 1u32..1000,
                width in 1u32..256,
                height in 1u32..256,
                format in prop::sample::select(vec![24u32, 32, 100]),
            ) {
                let input = format!("i={},s={},v={},f={}", id, width, height, format);
                let cmd = parse_control_data(input.as_bytes()).unwrap();
                assert_eq!(cmd.image_id, Some(id));
                assert_eq!(cmd.width, Some(width));
                assert_eq!(cmd.height, Some(height));
            }
        }
    }
}
