use image::{ImageBuffer, Rgb};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

mod toroid;
use crate::toroid::{NoiseConfig, ToroidNoise, ToroidTectonics, VoronoiConfig};
fn main() {
    let tectonics = ToroidTectonics::new(VoronoiConfig {
        width:2048., 
        height:2048., 
        sites_per:5, 
        seed:0 
    });
    let warp = ToroidNoise::new(NoiseConfig {
        width: 2048.,
        height: 2048.,
        frequency: 1.,
        scale: 512.,
        seed: 0
    });
    draw(tectonics, warp)
}

const SIZE: u32 = 2048;
fn draw(plates: ToroidTectonics, warp: ToroidNoise) {
    let buf: Vec<u8> = (0..SIZE).into_par_iter().flat_map(|y| {
        (0..SIZE).flat_map(|x| {
            plates.get_color_at(&warp, x as f64, y as f64).to_vec()
        }).collect::<Vec<u8>>()
    }).collect();
    ImageBuffer::<Rgb<u8>, _>::from_raw(SIZE, SIZE, buf)
        .unwrap().save("test.png").expect("To save image");
}
