use blip::image::Image;
use blip::Asset;

fn ball() -> Vec<u8> {
    let mut image = Image::new(4, 4);
    for y in 0..4i32 {
        for x in 0..4i32 {
            image.set(x, y, 255, 255, 255);
        }
    }
    image.encode_png()
}

pub fn generate() -> Vec<Asset> {
    vec![("images/ball.png", ball())]
}
