//! Wire format for two-device Rally (see `docs/multiplayer.md`).
//!
//! One fixed-size binary packet, host -> guest only: the ball, both
//! paddles, both scores, and which of the five [`crate::State`] values
//! the host is in (as a plain ordinal — this module doesn't depend on
//! `main.rs`'s `State` enum, so the mapping back is the caller's job).
//! Guest -> host traffic (paddle up/down) is plain JSON handled entirely
//! in `web/blip_net.js`; it never touches Rust, so it has no wire format
//! here.
//!
//! Deliberately not `serde`: this is nine fixed fields on a hot path
//! (packed and sent every frame the host is live), so a hand-rolled
//! little-endian layout keeps it allocation-free and trivially portable
//! to the JS side, which just needs to know the same 9-field, 4-bytes-
//! each order to lay out the matching `ArrayBuffer` view.

/// One frame of authoritative host state.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct NetState {
    /// The host's `State` as a plain ordinal: 0=Serve, 1=Play, 2=Point,
    /// 3=Over (Title is never sent — a guest only exists once a match has
    /// already started). Kept as a bare `i32` rather than an enum here so
    /// this module has no dependency on `main.rs`.
    pub phase: i32,
    pub ball_x: f32,
    pub ball_y: f32,
    pub ball_vx: f32,
    pub ball_vy: f32,
    pub lpad_y: f32,
    pub rpad_y: f32,
    pub score_l: i32,
    pub score_r: i32,
}

/// Packet size in bytes: 9 fields x 4 bytes, little-endian, no padding.
pub const NET_STATE_LEN: usize = 36;

/// Serialize into a fixed-size buffer for [`blip::web::net_send`].
pub fn pack_state(s: &NetState) -> [u8; NET_STATE_LEN] {
    let mut buf = [0u8; NET_STATE_LEN];
    buf[0..4].copy_from_slice(&s.phase.to_le_bytes());
    buf[4..8].copy_from_slice(&s.ball_x.to_le_bytes());
    buf[8..12].copy_from_slice(&s.ball_y.to_le_bytes());
    buf[12..16].copy_from_slice(&s.ball_vx.to_le_bytes());
    buf[16..20].copy_from_slice(&s.ball_vy.to_le_bytes());
    buf[20..24].copy_from_slice(&s.lpad_y.to_le_bytes());
    buf[24..28].copy_from_slice(&s.rpad_y.to_le_bytes());
    buf[28..32].copy_from_slice(&s.score_l.to_le_bytes());
    buf[32..36].copy_from_slice(&s.score_r.to_le_bytes());
    buf
}

/// Parse a buffer received from [`blip::web::net_poll`]. `None` if it's
/// not exactly [`NET_STATE_LEN`] bytes (a partial/garbled packet — the
/// caller should just keep showing the last good state rather than crash
/// or apply a bogus one).
pub fn unpack_state(bytes: &[u8]) -> Option<NetState> {
    if bytes.len() != NET_STATE_LEN {
        return None;
    }
    let i32_at = |o: usize| i32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    let f32_at = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    Some(NetState {
        phase: i32_at(0),
        ball_x: f32_at(4),
        ball_y: f32_at(8),
        ball_vx: f32_at(12),
        ball_vy: f32_at(16),
        lpad_y: f32_at(20),
        rpad_y: f32_at(24),
        score_l: i32_at(28),
        score_r: i32_at(32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NetState {
        NetState {
            phase: 1,
            ball_x: 240.5,
            ball_y: -12.25, // negative + fractional: exercises sign + mantissa bits together
            ball_vx: 450.0,
            ball_vy: -450.0,
            lpad_y: 0.0,
            rpad_y: 539.999,
            score_l: 6,
            score_r: 7,
        }
    }

    #[test]
    fn round_trips_byte_exact() {
        let s = sample();
        let bytes = pack_state(&s);
        assert_eq!(bytes.len(), NET_STATE_LEN);
        let back = unpack_state(&bytes).expect("valid packet");
        assert_eq!(back, s);
    }

    #[test]
    fn field_order_matches_the_documented_layout() {
        // A change here is a wire-format break for anything already
        // talking to a deployed host — this test exists to make that
        // change loud, not to forbid it.
        let s = NetState {
            phase: 3,
            ball_x: 1.0, ball_y: 2.0, ball_vx: 3.0, ball_vy: 4.0,
            lpad_y: 5.0, rpad_y: 6.0,
            score_l: 7, score_r: 8,
        };
        let bytes = pack_state(&s);
        assert_eq!(i32::from_le_bytes(bytes[0..4].try_into().unwrap()), 3);
        assert_eq!(f32::from_le_bytes(bytes[4..8].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2.0);
        assert_eq!(f32::from_le_bytes(bytes[12..16].try_into().unwrap()), 3.0);
        assert_eq!(f32::from_le_bytes(bytes[16..20].try_into().unwrap()), 4.0);
        assert_eq!(f32::from_le_bytes(bytes[20..24].try_into().unwrap()), 5.0);
        assert_eq!(f32::from_le_bytes(bytes[24..28].try_into().unwrap()), 6.0);
        assert_eq!(i32::from_le_bytes(bytes[28..32].try_into().unwrap()), 7);
        assert_eq!(i32::from_le_bytes(bytes[32..36].try_into().unwrap()), 8);
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(unpack_state(&[0u8; 35]), None);
        assert_eq!(unpack_state(&[0u8; 37]), None);
        assert_eq!(unpack_state(&[]), None);
    }

    #[test]
    fn survives_nan_and_infinity_without_panicking() {
        // The network can hand back garbage bytes that happen to decode to
        // NaN/inf for a float field (e.g. a torn packet) — unpack must not
        // panic; the caller decides whether to trust/clamp the result.
        let s = NetState {
            phase: 1,
            ball_x: f32::NAN, ball_y: f32::INFINITY, ball_vx: f32::NEG_INFINITY, ball_vy: 0.0,
            lpad_y: 0.0, rpad_y: 0.0, score_l: 0, score_r: 0,
        };
        let bytes = pack_state(&s);
        let back = unpack_state(&bytes).expect("valid length");
        assert!(back.ball_x.is_nan());
        assert!(back.ball_y.is_infinite() && back.ball_y > 0.0);
        assert!(back.ball_vx.is_infinite() && back.ball_vx < 0.0);
    }

    #[test]
    fn extreme_scores_round_trip() {
        let s = NetState { phase: 3, score_l: i32::MAX, score_r: i32::MIN, ..sample() };
        let back = unpack_state(&pack_state(&s)).unwrap();
        assert_eq!(back.score_l, i32::MAX);
        assert_eq!(back.score_r, i32::MIN);
    }
}
