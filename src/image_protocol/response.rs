// APC response formatting for the Kitty Graphics Protocol.
//
// Generates response strings to write back to the PTY, following the protocol spec:
//   ESC_G i=<id>[,I=<number>];<status> ESC\

/// Response kind for Kitty Graphics Protocol.
#[derive(Debug)]
pub enum ResponseKind<'a> {
    Ok,
    Error(&'a str, &'a str), // (code, message)
}

/// Format an APC graphics response.
/// Returns None if both IDs are zero/absent (no response needed per protocol).
pub fn format_response(
    image_id: Option<u32>,
    image_number: Option<u32>,
    kind: ResponseKind,
) -> Option<String> {
    let id = image_id.unwrap_or(0);
    let num = image_number.unwrap_or(0);

    // No response when both IDs are zero/absent
    if id == 0 && num == 0 {
        return None;
    }

    let id_part = if num > 0 && id > 0 {
        format!("i={},I={}", id, num)
    } else if num > 0 {
        format!("I={}", num)
    } else {
        format!("i={}", id)
    };

    let status = match kind {
        ResponseKind::Ok => "OK".to_string(),
        ResponseKind::Error(code, msg) => format!("{}:{}", code, msg),
    };

    Some(format!("\x1b_G{};{}\x1b\\", id_part, status))
}

/// Format a response with quiet mode handling.
/// q=0: always respond
/// q=1: suppress OK, still send errors
/// q=2: suppress everything
pub fn format_response_with_quiet(
    image_id: Option<u32>,
    image_number: Option<u32>,
    kind: ResponseKind,
    quiet: u8,
) -> Option<String> {
    match quiet {
        2 => None, // suppress everything
        1 => match kind {
            ResponseKind::Ok => None, // suppress OK
            ResponseKind::Error(_, _) => format_response(image_id, image_number, kind),
        },
        _ => format_response(image_id, image_number, kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_ok_with_id() {
        let resp = format_response(Some(4), None, ResponseKind::Ok);
        assert_eq!(resp, Some("\x1b_Gi=4;OK\x1b\\".to_string()));
    }

    #[test]
    fn response_ok_with_id_and_number() {
        let resp = format_response(Some(12), Some(4), ResponseKind::Ok);
        assert_eq!(resp, Some("\x1b_Gi=12,I=4;OK\x1b\\".to_string()));
    }

    #[test]
    fn response_error() {
        let resp = format_response(Some(1), None, ResponseKind::Error("ENOENT", "image not found"));
        assert_eq!(
            resp,
            Some("\x1b_Gi=1;ENOENT:image not found\x1b\\".to_string())
        );
    }

    #[test]
    fn response_suppressed_q1_ok() {
        let resp = format_response_with_quiet(Some(1), None, ResponseKind::Ok, 1);
        assert!(resp.is_none(), "q=1 suppresses OK");
    }

    #[test]
    fn response_not_suppressed_q1_error() {
        let resp = format_response_with_quiet(
            Some(1),
            None,
            ResponseKind::Error("ENODATA", "insufficient"),
            1,
        );
        assert!(resp.is_some(), "q=1 does not suppress errors");
    }

    #[test]
    fn response_suppressed_q2_error() {
        let resp = format_response_with_quiet(
            Some(1),
            None,
            ResponseKind::Error("ENODATA", "insufficient"),
            2,
        );
        assert!(resp.is_none(), "q=2 suppresses everything");
    }

    #[test]
    fn no_response_for_zero_ids() {
        let resp = format_response(None, None, ResponseKind::Ok);
        assert!(
            resp.is_none(),
            "no response when both IDs are zero/absent"
        );
    }

    #[test]
    fn response_with_only_number() {
        let resp = format_response(None, Some(5), ResponseKind::Ok);
        assert_eq!(resp, Some("\x1b_GI=5;OK\x1b\\".to_string()));
    }
}
