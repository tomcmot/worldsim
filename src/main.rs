use std::collections::HashMap;
use tiny_skia::*;
use noise::{Fbm, NoiseFn, Simplex, Vector2};
use image::{Rgb, RgbImage};
use voronoice::{self, BoundingBox, Point, VoronoiBuilder, VoronoiCell};

fn main() {
    let mut points = vec![
        Point{x:680.,  y:680.},
        Point{x:1360., y:680.},
        Point{x:2040., y:680.},
        Point{x:2720., y:680.},
        Point{x:3400., y:680.},

        Point{x:680.,  y:1360.},
        Point{x:1360., y:1360.},
        Point{x:2040., y:1360.},
        Point{x:2720., y:1360.},
        Point{x:3400., y:1360.},

        Point{x:680.,  y:2040.},
        Point{x:1360., y:2040.},
        Point{x:2040., y:2040.},
        Point{x:2720., y:2040.},
        Point{x:3400., y:2040.},

        Point{x:680.,  y:2720.},
        Point{x:1360., y:2720.},
        Point{x:2040., y:2720.},
        Point{x:2720., y:2720.},
        Point{x:3400., y:2720.},

        Point{x:680.,  y:3400.},
        Point{x:1360., y:3400.},
        Point{x:2040., y:3400.},
        Point{x:2720., y:3400.},
        Point{x:3400., y:3400.},
    ];
    for p in points.iter_mut() {
        p.x = p.x + produce_modifier();
        p.y = p.y + produce_modifier();
    }
    let mirrored = 
        points.iter()
        .flat_map(mirror_points)
        .collect::<Vec<Point>>();
    let diagram =
        VoronoiBuilder::default()
        .set_sites(mirrored)
        .set_bounding_box(BoundingBox::new(Point{x:2048.,y:2048.},2.*4096., 2.*4096.))
        .set_lloyd_relaxation_iterations(0)
        .build()
        .unwrap();
    let neighbors = find_neighbors(&diagram);
    let continents = pick_continents(points, neighbors);
    let d2 =
        diagram.iter_cells()
        .map(|c| {
            let can = canonical(c.site_position());
            to_cell(c, continents.contains(&can))
        }).collect::<Vec<_>>();
    draw_image(d2);
    save_image(diagram, continents);
}


fn warp_domain(noise: &Fbm<Simplex>, p: Vector2<f64>) -> Vector2<f64> {
    let offset_x = noise.get([p.x, p.y]);
    let offset_y = noise.get([p.x+ 5.2, p.y + 1.3]);
    Vector2 {
        x: p.x + offset_x,
        y: p.y + offset_y
    }
}

#[derive(Debug)]
struct Cell {
    site: Vector2<f64>,
    continent: bool,
}

fn to_cell(cell: VoronoiCell, continent: bool) -> Cell {
    let site = cell.site_position();
    Cell {
        site: Vector2 {
            x: 6. * site.x as f64 / 4096.,
            y: 6. * site.y as f64 / 4096.,
        },
        continent
    }
}

fn distance_sq(a:Vector2<f64>, b: Vector2<f64>) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn draw_image(diagram: Vec<Cell>) {
    let noise: Fbm<Simplex> = Fbm::new(0);
    println!("Starting to draw noise");
    let image = RgbImage::from_fn(4096, 4096, |x,y| {
        println!("handling: {} {}", x, y);
        let x = 6. * x as f64 / 4096.;
        let y = 6. * y as f64 / 4096.;
        let warped = warp_domain(&noise, Vector2{x,y});
        let cell = diagram.iter().min_by(|a, b| {
            f64::total_cmp(&distance_sq(warped, a.site), &distance_sq(warped, b.site))
        }).unwrap();
        if cell.continent {
            Rgb([100, 200, 60])
        } else {
            Rgb([60, 100, 200])
        }
    });
    println!("Saving noise");
    image.save("test2.png").expect("Image to save");
}

fn save_image(diagram: voronoice::Voronoi, continents: Vec<Point>) {
    println!("Generating image");
    let mut stroke_color = Paint::default();
    stroke_color.set_color_rgba8(0, 0, 0, 255);
    let mut pixmap = Pixmap::new(4096, 4096).unwrap();
    pixmap.fill(Color::from_rgba8(100, 100, 100, 255));
    let stroke = Stroke::default();
    println!("Drawing");
    for cell in diagram.iter_cells() {
        let mut path = PathBuilder::new();
        let mut started = false;
        for v in cell.iter_vertices() {
            if started {
                path.line_to(v.x as f32, v.y as f32);
            } else {
                path.move_to(v.x as f32, v.y as f32);
                started = true;
            }
        }
        path.close();
        let p = path.finish().unwrap();
        let fill_color = assign_color(cell, &continents);
        pixmap.fill_path(&p, &fill_color, FillRule::Winding, Transform::identity(), None);
        pixmap.stroke_path(&p, &stroke_color, &stroke, Transform::identity(), None);
    }
    println!("Saving");
    match pixmap.save_png("test.png") {
        Ok (_) => {},
        Err(e) => {
            println!("{}", e)
        }
    }
}

fn find_neighbors(diagram: &voronoice::Voronoi) -> HashMap<(i32, i32), Vec<Point>> {
    let mut neighbors : HashMap<(i32,i32), Vec<Point>> = HashMap::new();
    for c in diagram.iter_cells() {
        let cell = canonical(c.site_position()); 
        let key = (cell.x.floor() as i32, cell.y.floor() as i32);
        for n in c.iter_neighbors() {
            let neighbor = diagram.cell(n);
            let p = canonical(neighbor.site_position());
            match neighbors.get_mut(&key) {
                Some (points) =>
                    points.push(p),
                None => {neighbors.insert(key, vec![p]);}
            }
        }
    }
    neighbors
}

fn pick_continents(points: Vec<Point>, neighbors: HashMap<(i32, i32), Vec<Point>>) -> Vec<Point> {
    let start = points[rand::random_range(0..points.len())].clone();
    let mut continents: Vec<Point> = vec![start.clone()];
    let mut exclude = neighbors.get(&(start.x as i32, start.y as i32)).unwrap().clone();
    let num_continents = rand::random_range(7..11);
    while continents.len() < num_continents {
        let alltargets = points.iter().filter(|p| !continents.contains(p)).collect::<Vec<_>>();
        let targets = points.iter().filter(|p| !(continents.contains(p) || exclude.contains(p))).collect::<Vec<_>>();
        println!("Excluding {:?}", exclude);
        println!("Valid targets: {:?}", targets);
        if targets.is_empty() {
            let p = alltargets[rand::random_range(0..alltargets.len())].clone();
            continents.push(p);
        } else {
            let p = targets[rand::random_range(0..targets.len())].clone();
            let mut n = neighbors.get(&(p.x as i32, p.y as i32)).unwrap().clone();
            exclude.append(&mut n);
            continents.push(p);
        }
    }
    continents
}

fn produce_modifier() -> f64 {
    f64::floor(rand::random_range(-300.0 .. 300.))
}

fn mirror_points(p:&Point) -> Vec<Point> {
    vec![
        p.clone(),
        Point{x:p.x-4096., y:p.y},
        Point{x:p.x+4096., y:p.y},
        Point{x:p.x, y:p.y-4096.},
        Point{x:p.x, y:p.y+4096.},
        Point{x:p.x-4096., y:p.y-4096.},
        Point{x:p.x+4096., y:p.y+4096.},
        Point{x:p.x-4096., y:p.y+4096.},
        Point{x:p.x+4096., y:p.y-4096.}
    ]
}


fn assign_color<'a>(c:VoronoiCell<'a>, continents: &'a Vec<Point>) -> Paint<'a> {
    let pos = canonical(c.site_position());

    let color : Color = 
        if continents.contains(&pos) {
            Color::from_rgba8(100, 200, 60, 255)
        } else {
            Color::from_rgba8(60, 100, 200, 255)
        };

    let mut p = Paint::default();
    p.set_color(color);
    p
}


fn canonical(p:&Point) -> Point {
    Point {
        x: canonize(p.x),
        y: canonize(p.y)
    }
}

fn canonize(f:f64) -> f64 {
    if f < 0. {
        f + 4096.
    } else if f > 4096. {
        f - 4096.
    } else {
        f
    }
}