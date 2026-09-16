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

/// What a guest is willing to believe about a host's state packet.
///
/// [`unpack_state`] deliberately decodes anything of the right length,
/// bit patterns included — a torn packet must not panic, and the decoder
/// is not the place to decide policy. This is that policy, kept separate
/// so the wire format and the trust rules can be read (and tested) apart
/// from each other.
///
/// It matters because the guest applies these numbers directly to its
/// own `Game`: the host is simply believed. A host that sends `NaN` for
/// a ball coordinate poisons every comparison that coordinate later
/// takes part in (every `<`/`>` against `NaN` is false, so collision and
/// scoring logic silently stop firing), and one that sends a score of
/// `i32::MAX` draws a HUD no layout was designed for. Neither is
/// hypothetical: pairing is a QR code anyone can photograph, and nothing
/// authenticates the peer afterwards.
#[derive(Copy, Clone, Debug)]
pub struct Limits {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    /// Largest magnitude allowed for either velocity component.
    pub speed_max: f32,
    /// Highest score either side can legitimately hold.
    pub score_max: i32,
}

/// Validate and clamp a decoded packet, or reject it outright.
///
/// Two different responses, because the two kinds of wrongness differ:
///
/// - **Rejected** (`None`): values that cannot be a rounding error or a
///   frame of drift, and so indicate a packet that should not be
///   believed at all — any non-finite float, a phase that is not one of
///   the four sent on the wire, a score outside the possible range. The
///   caller keeps showing the last good frame, which is the same thing
///   it already does for a garbled or missing packet.
/// - **Clamped**: positions and velocities slightly outside the field.
///   A legitimate host really can report a ball a little past the edge
///   on the frame it scores, so rejecting on that would drop good
///   packets and stutter the guest's view for no gain.
pub fn sanitize_state(s: &NetState, l: &Limits) -> Option<NetState> {
    // Checked first and for every field: NaN fails all of the range
    // comparisons below, so a later `clamp` would silently pass it
    // through rather than catch it.
    for v in [s.ball_x, s.ball_y, s.ball_vx, s.ball_vy, s.lpad_y, s.rpad_y] {
        if !v.is_finite() {
            return None;
        }
    }
    if !(0..=3).contains(&s.phase) {
        return None;
    }
    if !(0..=l.score_max).contains(&s.score_l) || !(0..=l.score_max).contains(&s.score_r) {
        return None;
    }
    Some(NetState {
        phase: s.phase,
        ball_x: s.ball_x.clamp(l.x_min, l.x_max),
        ball_y: s.ball_y.clamp(l.y_min, l.y_max),
        ball_vx: s.ball_vx.clamp(-l.speed_max, l.speed_max),
        ball_vy: s.ball_vy.clamp(-l.speed_max, l.speed_max),
        lpad_y: s.lpad_y.clamp(l.y_min, l.y_max),
        rpad_y: s.rpad_y.clamp(l.y_min, l.y_max),
        score_l: s.score_l,
        score_r: s.score_r,
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

    fn limits() -> Limits {
        // Same shape as main.rs's NET_LIMITS, with round numbers.
        Limits { x_min: -48.0, x_max: 528.0, y_min: -20.0, y_max: 588.0, speed_max: 900.0, score_max: 7 }
    }

    #[test]
    fn sanitize_passes_a_normal_packet_through_untouched() {
        let s = sample();
        assert_eq!(sanitize_state(&s, &limits()), Some(s));
    }

    #[test]
    fn sanitize_rejects_every_non_finite_float_field() {
        // Each field separately: a single `is_finite` check written
        // against the wrong variable would still pass a test that only
        // poisoned one of them.
        let base = sample();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for (name, s) in [
                ("ball_x", NetState { ball_x: bad, ..base }),
                ("ball_y", NetState { ball_y: bad, ..base }),
                ("ball_vx", NetState { ball_vx: bad, ..base }),
                ("ball_vy", NetState { ball_vy: bad, ..base }),
                ("lpad_y", NetState { lpad_y: bad, ..base }),
                ("rpad_y", NetState { rpad_y: bad, ..base }),
            ] {
                assert_eq!(sanitize_state(&s, &limits()), None, "{name} = {bad} should be rejected");
            }
        }
    }

    #[test]
    fn sanitize_rejects_impossible_scores() {
        let b = sample();
        assert_eq!(sanitize_state(&NetState { score_l: -1, ..b }, &limits()), None);
        assert_eq!(sanitize_state(&NetState { score_r: 8, ..b }, &limits()), None);
        assert_eq!(sanitize_state(&NetState { score_l: i32::MAX, ..b }, &limits()), None);
        assert_eq!(sanitize_state(&NetState { score_r: i32::MIN, ..b }, &limits()), None);
        // The winning score itself is legitimate and must survive.
        assert!(sanitize_state(&NetState { score_l: 7, score_r: 0, ..b }, &limits()).is_some());
    }

    #[test]
    fn sanitize_rejects_a_phase_that_is_not_on_the_wire() {
        let b = sample();
        for p in [-1, 4, 99, i32::MAX, i32::MIN] {
            assert_eq!(sanitize_state(&NetState { phase: p, ..b }, &limits()), None, "phase {p}");
        }
        for p in 0..=3 {
            assert!(sanitize_state(&NetState { phase: p, ..b }, &limits()).is_some(), "phase {p}");
        }
    }

    #[test]
    fn sanitize_clamps_rather_than_rejects_out_of_field_positions() {
        // A real host reports a ball past the edge on the frame it
        // scores, so these must survive — bounded, not dropped.
        let l = limits();
        let s = NetState { ball_x: 1.0e30, ball_y: -1.0e30, lpad_y: 9999.0, rpad_y: -9999.0, ..sample() };
        let out = sanitize_state(&s, &l).expect("clamped, not rejected");
        assert_eq!(out.ball_x, l.x_max);
        assert_eq!(out.ball_y, l.y_min);
        assert_eq!(out.lpad_y, l.y_max);
        assert_eq!(out.rpad_y, l.y_min);
    }

    #[test]
    fn sanitize_clamps_runaway_velocity_both_directions() {
        let l = limits();
        let out = sanitize_state(&NetState { ball_vx: 1.0e12, ball_vy: -1.0e12, ..sample() }, &l).unwrap();
        assert_eq!(out.ball_vx, l.speed_max);
        assert_eq!(out.ball_vy, -l.speed_max);
    }

    #[test]
    fn sanitize_survives_arbitrary_bytes_off_the_wire() {
        // The end-to-end property that matters: whatever 36 bytes arrive,
        // the guest either gets a usable state or nothing — never a panic
        // and never a NaN it will go on to compare against.
        let mut seed = 0x12345678u32;
        for _ in 0..20000 {
            let mut buf = [0u8; NET_STATE_LEN];
            for b in buf.iter_mut() {
                // xorshift — deterministic, no dev-dependency needed.
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                *b = (seed >> 24) as u8;
            }
            if let Some(s) = unpack_state(&buf).and_then(|s| sanitize_state(&s, &limits())) {
                assert!(s.ball_x.is_finite() && s.ball_y.is_finite());
                assert!(s.ball_vx.is_finite() && s.ball_vy.is_finite());
                assert!(s.lpad_y.is_finite() && s.rpad_y.is_finite());
                assert!((0..=3).contains(&s.phase));
                assert!((0..=7).contains(&s.score_l) && (0..=7).contains(&s.score_r));
            }
        }
    }

    #[test]
    fn extreme_scores_round_trip() {
        let s = NetState { phase: 3, score_l: i32::MAX, score_r: i32::MIN, ..sample() };
        let back = unpack_state(&pack_state(&s)).unwrap();
        assert_eq!(back.score_l, i32::MAX);
        assert_eq!(back.score_r, i32::MIN);
    }
}
