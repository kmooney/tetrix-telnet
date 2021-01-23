use tetrix::shape::{Shape, Orientation};

#[derive(Debug)]
pub struct ShapeRep {
    pub bytes: &'static [u8],
    pub width: u8,
    pub color_code: &'static str
}

pub fn shape_rep(s: Shape, o: Orientation) -> ShapeRep {
    match s {
        Shape::Eye => match o {
            Orientation::Up => ShapeRep {bytes: b"* ** ** ** ** ** ** ** *", width: 3, color_code: "[1;31m"},
            Orientation::Down => ShapeRep {bytes: b"* ** ** ** ** ** ** ** *", width: 3, color_code: "[1;31m"},
            Orientation::Left => ShapeRep {bytes: b"* * * * * * * ** * * * * * * *", width: 15, color_code: "[1;31m"},
            Orientation::Right => ShapeRep {bytes: b"* * * * * * * ** * * * * * * *", width: 15, color_code: "[1;31m"},
        },
        Shape::El => match o {
            Orientation::Up    => ShapeRep {bytes: b"* *    * *    * *    * *    * * * ** * * *", width: 7, color_code: "[1; 34m"},
            Orientation::Right => ShapeRep {bytes: b"* * * * * ** * * * * ** *        * *        ", width: 11, color_code: "[1; 34m"},
            Orientation::Down  => ShapeRep {bytes: b"* * * ** * * *    * *    * *    * *    * *", width: 7, color_code: "[1; 34m"},
            Orientation::Left  => ShapeRep {bytes: b"        * *        * ** * * * * ** * * * * *", width: 11, color_code: "[1; 34m"},            
        },
        Shape::ElInv => match o {
            Orientation::Up    => ShapeRep {bytes: b"    * *    * *    * *    * ** * * ** * * *", width: 7, color_code: "[1; 35m"},
            Orientation::Left  => ShapeRep {bytes: b"* * * * * ** * * * * *        * *        * *", width: 11, color_code: "[1; 35m"},
            Orientation::Down  => ShapeRep {bytes: b"* * * ** * * ** *    * *    * *    * *    ", width: 7, color_code: "[1; 35m"},
            Orientation::Right => ShapeRep {bytes: b"* *        * *        * * * * * ** * * * * *", width: 11, color_code: "[1; 35m"},
        },
        Shape::Zee => match o {
            Orientation::Up    => ShapeRep {bytes: b"    * *    * ** * * ** * * ** *    * *    ", width: 7, color_code: "[1; 36m"},
            Orientation::Left  => ShapeRep {bytes: b"* * * *    * * * *        * * * *    * * * *", width: 11, color_code: "[1; 36m"},
            Orientation::Down  => ShapeRep {bytes: b"    * *    * ** * * ** * * ** *    * *    ", width: 7, color_code: "[1; 36m"},
            Orientation::Right => ShapeRep {bytes: b"* * * *    * * * *        * * * *    * * * *", width: 11, color_code: "[1; 36m"},
        },
        Shape::ZeeInv => match o {
            Orientation::Left   => ShapeRep {bytes: b"    * * * *    * * * ** * * *    * * * *    ", width: 11, color_code: "[1; 37m"},
            Orientation::Right  => ShapeRep {bytes: b"    * * * *    * * * ** * * *    * * * *    ", width: 11, color_code: "[1; 37m"},
            Orientation::Up     => ShapeRep {bytes: b"* *    * *    * * * ** * * *    * *    * *", width: 7, color_code: "[1; 37m"},
            Orientation::Down   => ShapeRep {bytes: b"* *    * *    * * * ** * * *    * *    * *", width: 7, color_code: "[1; 37m"},
        },
        Shape::Square => match o {
            _ => ShapeRep {bytes: b"* * * ** * * ** * * ** * * *", width: 7, color_code: "[1; 33m"}
        },
        Shape::Tee => match o {
            Orientation::Down    => ShapeRep {bytes: b"* * * * * ** * * * * *    * *        * *    ", width: 11, color_code: "[0; 34m"},
            Orientation::Left  => ShapeRep {bytes: b"    * *    * ** * * ** * * *    * *    * *", width: 7, color_code: "[0; 34m"},
            Orientation::Up  => ShapeRep {bytes: b"    * *        * *    * * * * * ** * * * * *", width: 11, color_code: "[0; 34m"},
            Orientation::Right => ShapeRep {bytes: b"* *    * *    * * * ** * * ** *    * *   ", width: 7, color_code: "[0; 34m"}
        }
    }
}