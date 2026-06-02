use std::{cmp::Ordering, collections::HashMap, f64::consts::TAU, hash::Hash};

use noise::{NoiseFn, Simplex};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use serde::{Deserialize, Serialize};
use voronoice::{BoundingBox, Point, VoronoiCell};

#[derive(Serialize, Deserialize)]
pub struct NoiseConfig {
    pub frequency: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    pub seed: u32
}
pub struct ToroidNoise {
    noise_dx: Simplex,
    noise_dy: Simplex,
    width: f64,
    height: f64,
    frequency: f64,
    scale: f64,
}

impl ToroidNoise {
    pub fn new(config: NoiseConfig) -> Self {
        let noise_dx = Simplex::new(config.seed);
        let noise_dy = Simplex::new(config.seed+1);
        ToroidNoise { noise_dx, noise_dy, width:config.width, height:config.height, frequency:config.frequency, scale:config.scale }
    }

    fn sample(&self, noise: &Simplex, x: f64, y: f64) -> f64 {
        let ax = (x/self.width) * TAU * self.frequency;
        let ay = (y/self.height) * TAU * self.frequency;
        noise.get([ax.cos(), ax.sin(), ay.cos(), ay.sin()])
    }

    pub fn warp(&self, x: f64, y:f64) -> Vector {
        let dx = self.sample(&self.noise_dx, x, y) * self.scale;
        let dy = self.sample(&self.noise_dy, x, y) * self.scale;
        Vector{x:x+dx, y:y+dy}
    }
}

#[derive(Serialize, Deserialize)]
pub struct VoronoiConfig {
    pub width: f64,
    pub height: f64,
    pub sites_per: u8,
    pub seed: u64,
}

#[derive(Copy, Clone)]
pub enum PlateType {
    Ocean,
    Contintent,
}

#[derive(PartialEq, Debug)]
pub struct Vector {
    x: f64,
    y: f64
}

impl Eq for Vector {
    
}
impl Hash for Vector{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

pub struct Line {
    start: Vector,
    end: Vector,
}
pub struct ToroidTectonics {
    sites: Vec<Vector>,
    mirrors: Vec<Point>,
    neighbors: Vec<Vec<usize>>,
    edges: Vec<Line>,
    motion: Vec<Vector>,
    type_: Vec<PlateType>,
    width: f64,
}


impl ToroidTectonics {
    pub fn new(config: VoronoiConfig) -> Self {
        let dx = config.width / config.sites_per as f64;
        let dy = config.height / config.sites_per as f64;
        let mut rng = SmallRng::seed_from_u64(config.seed);
        let sites = (0..config.sites_per).into_iter()
            .flat_map(|x| {
                (0..config.sites_per).into_iter()
                .map(|y| {
                    let x = x as f64 * dx + dx/2. + rng.random_range(-dx/3. .. dx/3.);
                    let y = y as f64 * dy + dy/2. + rng.random_range(-dy/3. ..dy/3.);
                    Vector{x, y}
                })
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        println!("site length {:?}", sites.len());
        let motion = (0..sites.len()).into_iter().map(|_| {
            let x: f64 = 6. * (rng.random::<f64>() - 0.5);
            let y: f64 = 6. * (rng.random::<f64>() - 0.5);
            Vector { x, y}
        }).collect::<Vec<Vector>>();
        let mirrors: Vec<Point> = sites.iter().flat_map(mirror_points(config.width)).collect();
        println!("mirror length: {:?}", mirrors.len());
        let diagram = 
            voronoice::VoronoiBuilder::default()
            .set_sites(mirrors.clone())
            .set_lloyd_relaxation_iterations(0)
            .set_bounding_box(BoundingBox::new(Point{
                x:config.width/2., 
                y: config.height/2.}, 
                config.width*2., 
                config.height*2.))
            .build()
            .unwrap();
        let mut cells: HashMap<Vector, Vec<VoronoiCell>> = HashMap::new();
        diagram.iter_cells().for_each(|cell| {
            let p = cell.site_position();
            let k = canonical(p, config.width);
            if let Some(v) = cells.get_mut(&k) {
                v.push(cell)
            } else {
                cells.insert(k, vec![cell]);
            }
        });
        println!("cells length: {}", diagram.cells().len());
        let neighbors = sites.iter().map(|v| {
            if let Some (n) = cells.get(v) {
                n.iter().flat_map(|c| {
                    c.iter_neighbors().map(|n| {
                        let cn = canonical(diagram.cell(n).site_position(), config.width);
                        println!("{:?}", cn);
                        sites.iter().position(|s| 
                            s.x.floor().total_cmp(&cn.x.floor()) == Ordering::Equal && s.y.floor().total_cmp(&cn.y.floor()) == Ordering::Equal
                        ).unwrap()
                    })
                }).collect()
            } else {
                vec![]
            }
        }).collect();
        let type_ = plate_type(&mut rng, &sites, &neighbors);
        ToroidTectonics { sites, mirrors, neighbors, edges: vec![], motion, type_, width: config.width}
    }

    pub fn get_color_at(&self, warp: &ToroidNoise, x: f64, y: f64) -> [u8;3] {
        let p = warp.warp(x, y);
        let point = self.mirrors.iter().min_by(|a,b| {
            distance_sq(&p, a).total_cmp(&distance_sq(&p, b))
        }).unwrap();
        let (i,_) = self.sites.iter().enumerate().find(| (_,a)| {
            let cn = canonical(point, self.width);
            a.x.floor().total_cmp(&cn.x.floor()) == Ordering::Equal && a.y.floor().total_cmp(&cn.y.floor()) == Ordering::Equal
        }).unwrap();
        color_of(self.type_[i])
    }
}

fn mirror_points(size: f64) -> impl FnMut(&Vector) -> Vec<Point> {
    move |p| {
        vec![
            Point{x:p.x-size, y:p.y},
            Point{x:p.x+size, y:p.y},
            Point{x:p.x, y:p.y-size},
            Point{x:p.x, y:p.y+size},
            Point{x:p.x-size, y:p.y-size},
            Point{x:p.x+size, y:p.y+size},
            Point{x:p.x-size, y:p.y+size},
            Point{x:p.x+size, y:p.y-size},
            Point{x:p.x, y: p.y}
        ]
    }
}

fn canonical(p:&Point, bound: f64) -> Vector {
    Vector {
        x: canonize(p.x, bound),
        y: canonize(p.y, bound)
    }
}

fn canonize(f:f64, bound: f64) -> f64 {
    if f < 0. {
        f + bound
    } else if f > bound {
        f - bound
    } else {
        f
    }
}

// TODO: smarter placement by checking neighbors
fn plate_type(rng : &mut SmallRng, points: &Vec<Vector>, neighbors: &Vec<Vec<usize>>) -> Vec<PlateType> {
    let mut continents: Vec<PlateType> = vec![PlateType::Ocean; points.len()];
    let start = rng.random_range(0..points.len());
    continents[start] = PlateType::Contintent;
    let num_continents = rng.random_range(7..11);
    let mut assigned = vec![start];
    while assigned.len() < num_continents {
        let next = rng.random_range(0..points.len());
        if !assigned.contains(&next) {
            continents[next] = PlateType::Contintent;
            assigned.push(next);
        }
    }
    continents
}

fn distance_sq(a:&Vector, b: &Point) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}

fn color_of(p: PlateType) -> [u8;3] {
    match p {
        PlateType::Ocean => [60, 100, 200],
        PlateType::Contintent => [100, 200, 60],
    }
}