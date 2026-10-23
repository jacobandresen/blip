//! BASIC bindings and browser exports for shared engine math.

macro_rules! functions {
    ($( $name:ident => $export:ident($($arg:ident),+) $body:block );+ $(;)?) => {
        $(
            #[no_mangle]
            pub extern "C" fn $export($($arg: f64),+) -> f64 $body
        )+

        pub fn contains(name: &str) -> bool {
            matches!(name, $(stringify!($name))|+)
        }

        pub fn call(name: &str, args: &[f64]) -> Option<Result<f64, String>> {
            match name {
                $( stringify!($name) => {
                    let arity = [$(stringify!($arg)),+].len();
                    Some(if args.len() != arity {
                        Err(format!("{name} needs {arity} arguments"))
                    } else {
                        let mut values = args.iter().copied();
                        $(let $arg = values.next().unwrap();)+
                        Ok($export($($arg),+))
                    })
                }, )+
                _ => None,
            }
        }
    };
}

functions! {
    MIN => blip_basic_math_min(a, b) { a.min(b) };
    MAX => blip_basic_math_max(a, b) { a.max(b) };
    ATN => blip_basic_math_atn(value) { value.atan() };
    ATAN2 => blip_basic_math_atan2(y, x) { y.atan2(x) };
    CLAMP => blip_basic_math_clamp(v, lo, hi) { blip::clamp(v as f32, lo as f32, hi as f32) as f64 };
    LERP => blip_basic_math_lerp(a, b, t) { blip::lerp(a as f32, b as f32, t as f32) as f64 };
    WRAP => blip_basic_math_wrap(v, lo, hi) { blip::wrap(v as f32, lo as f32, hi as f32) as f64 };
    DISTANCE => blip_basic_math_distance(x0, y0, x1, y1) { blip::distance(x0 as f32, y0 as f32, x1 as f32, y1 as f32) as f64 };
    ANGLE_DIFF => blip_basic_math_angle_diff(a, b) { blip::angle_diff(a as f32, b as f32) as f64 };
    SMOOTHSTEP01 => blip_basic_math_smoothstep01(t) { blip::smoothstep01(t as f32) as f64 };
    EASE_OUT_CUBIC => blip_basic_math_ease_out_cubic(t) { blip::ease_out_cubic(t as f32) as f64 };
    SCATTER => blip_basic_math_scatter(k, salt) { blip::scatter(k as usize, salt as u32) as f64 };
    RECTS_OVERLAP => blip_basic_math_rects_overlap(x0, y0, w0, h0, x1, y1, w1, h1) {
        blip::rects_overlap(x0 as f32, y0 as f32, w0 as f32, h0 as f32, x1 as f32, y1 as f32, w1 as f32, h1 as f32) as u8 as f64
    };
    SEGMENT_CIRCLE_OVERLAP => blip_basic_math_segment_circle_overlap(x0, y0, x1, y1, cx, cy, radius) {
        blip::segment_circle_overlap(x0 as f32, y0 as f32, x1 as f32, y1 as f32, cx as f32, cy as f32, radius as f32) as u8 as f64
    };
    SEGMENT_RECT_OVERLAP => blip_basic_math_segment_rect_overlap(x0, y0, x1, y1, rx, ry, rw, rh) {
        blip::segment_rect_overlap(x0 as f32, y0 as f32, x1 as f32, y1 as f32, rx as f32, ry as f32, rw as f32, rh as f32) as u8 as f64
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_use_native_precision_and_collision_boundaries() {
        assert_eq!(call("MIN", &[-3.0, 2.0]), Some(Ok(-3.0)));
        assert_eq!(call("MAX", &[-3.0, 2.0]), Some(Ok(2.0)));
        assert_eq!(call("ATN", &[1.0]), Some(Ok(std::f64::consts::FRAC_PI_4)));
        assert_eq!(call("ATAN2", &[1.0, -1.0]), Some(Ok(3.0 * std::f64::consts::FRAC_PI_4)));
        assert_eq!(call("CLAMP", &[8.0, -2.0, 3.0]), Some(Ok(3.0)));
        assert_eq!(call("DISTANCE", &[-2.0, -3.0, 1.0, 1.0]), Some(Ok(5.0)));
        assert_eq!(call("WRAP", &[-21.0, 0.0, 10.0]), Some(Ok(9.0)));
        assert_eq!(call("LERP", &[0.1, 0.9, 0.3]), Some(Ok(blip::lerp(0.1, 0.9, 0.3) as f64)));
        assert_eq!(call("RECTS_OVERLAP", &[0.0, 0.0, 2.0, 2.0, 2.0, 0.0, 1.0, 1.0]), Some(Ok(0.0)));
        assert_eq!(call("SEGMENT_CIRCLE_OVERLAP", &[0.0, 0.0, 10.0, 0.0, 5.0, 1.0, 1.0]), Some(Ok(1.0)));
        assert_eq!(call("SEGMENT_RECT_OVERLAP", &[0.0, 5.0, 20.0, 5.0, 8.0, 2.0, 2.0, 6.0]), Some(Ok(1.0)));
        assert_eq!(call("SCATTER", &[17.0, u32::MAX as f64]), Some(Ok(blip::scatter(17, u32::MAX) as f64)));
    }

    #[test]
    fn bad_arity_is_an_error_and_array_names_stay_available() {
        assert_eq!(call("CLAMP", &[1.0]), Some(Err("CLAMP needs 3 arguments".into())));
        assert_eq!(call("SCATTER", &[]), Some(Err("SCATTER needs 2 arguments".into())));
        assert_eq!(call("BODY_X", &[1.0]), None);
        assert_eq!(call("BRAWLER_OPPONENT", &[0.0, 0.0]), None);
    }
}
