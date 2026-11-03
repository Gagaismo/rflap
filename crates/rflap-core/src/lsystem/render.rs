//! The renderer (JFLAP `gui.lsystem.Renderer`): walks the symbols with a turtle and
//! collects what it draws.
//!
//! JFLAP makes two passes: one to measure the drawing and one to paint it. Both are
//! run here so that the bounds, which are the visited positions of the first pass, and
//! the shapes of the second are the ones JFLAP would produce.

use serde::{Deserialize, Serialize};

use super::Rgb;
use super::turtle::{Matrix, Turtle, java_double_to_string, java_parse_double};

/// A stroked line the turtle drew with `g`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub color: Rgb,
    pub width: f64,
}

/// A filled polygon (`{ ... }`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub points: Vec<(f64, f64)>,
    pub fill: Rgb,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Line(Segment),
    Polygon(Polygon),
}

/// A rectangle in drawing coordinates (y grows downwards).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the turtle drew, in paint order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    pub shapes: Vec<Shape>,
    /// Every position the turtle visited, including moves that did not draw.
    pub bounds: Bounds,
}

impl Drawing {
    pub fn segments(&self) -> impl Iterator<Item = &Segment> {
        self.shapes.iter().filter_map(|s| match s {
            Shape::Line(l) => Some(l),
            Shape::Polygon(_) => None,
        })
    }

    pub fn polygons(&self) -> impl Iterator<Item = &Polygon> {
        self.shapes.iter().filter_map(|s| match s {
            Shape::Polygon(p) => Some(p),
            Shape::Line(_) => None,
        })
    }

    /// SVG of the drawing. JFLAP paints into an image of the bounds plus five pixels on
    /// every side; the view box is that image.
    pub fn to_svg(&self, background: Option<&str>) -> String {
        let width = self.bounds.width.trunc() + 10.0;
        let height = self.bounds.height.trunc() + 10.0;
        let (x, y) = (self.bounds.x - 5.0, self.bounds.y - 5.0);
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{x} {y} {w} {h}\">\n",
            w = num(width),
            h = num(height),
            x = num(x),
            y = num(y),
        );
        if let Some(bg) = background {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
                num(x),
                num(y),
                num(width),
                num(height),
                bg
            ));
        }
        for shape in &self.shapes {
            match shape {
                Shape::Line(s) => out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"square\"/>\n",
                    num(s.x1),
                    num(s.y1),
                    num(s.x2),
                    num(s.y2),
                    s.color.hex(),
                    num(s.width.max(0.0)),
                )),
                Shape::Polygon(p) => {
                    let points: Vec<String> = p
                        .points
                        .iter()
                        .map(|(px, py)| format!("{},{}", num(*px), num(*py)))
                        .collect();
                    out.push_str(&format!(
                        "<polygon points=\"{}\" fill=\"{}\"/>\n",
                        points.join(" "),
                        p.fill.hex()
                    ));
                }
            }
        }
        out.push_str("</svg>");
        out
    }
}

/// A number with at most three decimals and no trailing zeros.
fn num(v: f64) -> String {
    let rounded = (v * 1000.0).round() / 1000.0;
    let mut s = format!("{rounded:.3}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" { "0".to_string() } else { s }
}

/// How to draw an expansion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawOptions {
    /// Seed of the generator that picks among several replacements of a symbol.
    pub seed: i64,
    /// The initial view rotation in degrees (JFLAP's pitch, roll and yaw spinners).
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
    /// SVG background colour; JFLAP's image is transparent.
    pub background: Option<String>,
}

impl Default for DrawOptions {
    fn default() -> Self {
        DrawOptions {
            seed: 0,
            pitch: 0.0,
            roll: 0.0,
            yaw: 0.0,
            background: None,
        }
    }
}

#[derive(Clone, Copy)]
enum Handler {
    Move { pen_down: bool },
    Turn { clockwise: bool },
    Pitch { down: bool },
    Roll { right: bool },
    Push,
    Pop,
    Width { increment: bool },
    BeginPolygon,
    ClosePolygon,
    Reverse,
    Hue { polygon: bool, add: bool },
    Color,
    PolygonColor,
    Angle,
    LineWidth,
    LineIncrement,
    Distance,
    HueChange,
}

fn handler(name: &str) -> Option<Handler> {
    Some(match name {
        "g" => Handler::Move { pen_down: true },
        "f" => Handler::Move { pen_down: false },
        "+" => Handler::Turn { clockwise: true },
        "-" => Handler::Turn { clockwise: false },
        "&" => Handler::Pitch { down: true },
        "^" => Handler::Pitch { down: false },
        "/" => Handler::Roll { right: true },
        "*" => Handler::Roll { right: false },
        "[" => Handler::Push,
        "]" => Handler::Pop,
        "!" => Handler::Width { increment: true },
        "~" => Handler::Width { increment: false },
        "{" => Handler::BeginPolygon,
        "}" => Handler::ClosePolygon,
        "%" => Handler::Reverse,
        "#" => Handler::Hue {
            polygon: false,
            add: true,
        },
        "@" => Handler::Hue {
            polygon: false,
            add: false,
        },
        "##" => Handler::Hue {
            polygon: true,
            add: true,
        },
        "@@" => Handler::Hue {
            polygon: true,
            add: false,
        },
        "color" => Handler::Color,
        "polygonColor" => Handler::PolygonColor,
        "angle" | "angleIncrement" => Handler::Angle,
        "lineWidth" => Handler::LineWidth,
        "lineIncrement" => Handler::LineIncrement,
        "distance" => Handler::Distance,
        "hueChange" => Handler::HueChange,
        _ => return None,
    })
}

/// A pass over the symbols; `Err(())` stands for the exceptions JFLAP swallows.
struct Pass {
    drawing: bool,
    turtle: Turtle,
    stack: Vec<Turtle>,
    polygon: Option<Vec<(f64, f64)>>,
    shapes: Vec<Shape>,
    min: (f64, f64),
    max: (f64, f64),
}

impl Pass {
    fn new(drawing: bool, matrix: Matrix) -> Self {
        let turtle = Turtle::new(matrix);
        let p = turtle.position();
        Pass {
            drawing,
            turtle,
            stack: Vec::new(),
            polygon: None,
            shapes: Vec::new(),
            min: p,
            max: p,
        }
    }

    fn visit(&mut self) -> (f64, f64) {
        let p = self.turtle.position();
        self.min = (self.min.0.min(p.0), self.min.1.min(p.1));
        self.max = (self.max.0.max(p.0), self.max.1.max(p.1));
        p
    }

    fn number(arg: Option<&str>) -> Result<f64, ()> {
        java_parse_double(arg.ok_or(())?).ok_or(())
    }

    fn amount(&self, arg: &str) -> Result<f64, ()> {
        self.turtle.value_of(arg)
    }

    fn handle(&mut self, h: Handler, arg: Option<&str>) -> Result<(), ()> {
        match h {
            Handler::Move { pen_down } => {
                let before = self.turtle.position();
                match arg {
                    None => self.turtle.go_dir(true),
                    Some(a) => {
                        let d = self.amount(a)?;
                        self.turtle.go(d);
                    }
                }
                let after = self.visit();
                if self.drawing && pen_down {
                    match &mut self.polygon {
                        None => self.shapes.push(Shape::Line(Segment {
                            x1: before.0,
                            y1: before.1,
                            x2: after.0,
                            y2: after.1,
                            color: self.turtle.color,
                            width: self.turtle.line_width.max(0.0),
                        })),
                        Some(points) => points.push(after),
                    }
                }
            }
            Handler::Turn { clockwise } => match arg {
                None => self.turtle.turn_dir(clockwise),
                Some(a) => {
                    let d = self.amount(a)?;
                    self.turtle.turn(if clockwise { -d } else { d });
                }
            },
            Handler::Pitch { down } => match arg {
                None => self.turtle.pitch_dir(down),
                Some(a) => {
                    let d = self.amount(a)?;
                    self.turtle.pitch(if down { d } else { -d });
                }
            },
            Handler::Roll { right } => match arg {
                None => self.turtle.roll_dir(right),
                Some(a) => {
                    let d = self.amount(a)?;
                    self.turtle.roll(if right { -d } else { d });
                }
            },
            Handler::Push => self.stack.push(self.turtle.clone()),
            Handler::Pop => {
                if let Some(t) = self.stack.pop() {
                    self.turtle = t;
                }
            }
            Handler::Width { increment } => {
                let d = match arg {
                    None => self.turtle.width_increment,
                    Some(a) => self.amount(a)?,
                };
                self.turtle
                    .change_line_width(if increment { d } else { -d });
            }
            Handler::BeginPolygon => {
                if self.drawing && self.polygon.is_none() {
                    self.polygon = Some(vec![self.turtle.position()]);
                }
            }
            Handler::ClosePolygon => {
                if self.drawing {
                    let points = self.polygon.take().ok_or(())?;
                    self.shapes.push(Shape::Polygon(Polygon {
                        points,
                        fill: self.turtle.polygon_color,
                    }));
                }
            }
            Handler::Reverse => self.turtle.turn(180.0),
            Handler::Hue { polygon, add } => {
                if self.drawing {
                    let d = match arg {
                        None => self.turtle.hue_change,
                        Some(a) => self.amount(a)?,
                    };
                    let d = if add { d } else { -d };
                    if polygon {
                        self.turtle.change_polygon_hue(d);
                    } else {
                        self.turtle.change_hue(d);
                    }
                }
            }
            Handler::Color => {
                if self.drawing {
                    self.turtle.set_color(arg.ok_or(())?)?;
                }
            }
            Handler::PolygonColor => {
                if self.drawing {
                    self.turtle.set_polygon_color(arg.ok_or(())?)?;
                }
            }
            Handler::Angle => self.turtle.set_angle_change(Self::number(arg)?),
            Handler::LineWidth => {
                if self.drawing {
                    self.turtle.set_line_width(Self::number(arg)?);
                }
            }
            Handler::LineIncrement => self.turtle.set_line_increment(Self::number(arg)?),
            Handler::Distance => self.turtle.distance = Self::number(arg)?,
            Handler::HueChange => self.turtle.set_hue_change(Self::number(arg)?),
        }
        Ok(())
    }

    /// `Renderer.assign`: `name=value`.
    fn assign(&mut self, name: &str, value: &str) {
        let mut text = value.to_string();
        if name != "color"
            && name != "polygonColor"
            && let Ok(v) = self.turtle.assign(name, value)
        {
            text = java_double_to_string(v);
        }
        if let Some(h) = handler(name) {
            let _ = self.handle(h, Some(&text));
        }
    }

    fn run(&mut self, symbols: &[String], parameters: &[(&str, &str)]) {
        for (name, value) in parameters {
            self.assign(name, value);
        }
        for symbol in symbols {
            if let Some(h) = handler(symbol) {
                let _ = self.handle(h, None);
                continue;
            }
            if let Some(n) = symbol.find('=') {
                self.assign(&symbol[..n], &symbol[n + 1..]);
            }
            if let (Some(open), Some(close)) = (symbol.find('('), symbol.rfind(')'))
                && open < close
                && let Some(h) = handler(&symbol[..open])
            {
                let _ = self.handle(h, Some(&symbol[open + 1..close]));
            }
        }
    }
}

/// Draws `symbols` with the turtle, applying `parameters` first (in the order given).
pub(crate) fn render(
    symbols: &[String],
    parameters: &[(&str, &str)],
    options: &DrawOptions,
) -> Drawing {
    let mut view = Matrix::default();
    view.pitch(options.pitch);
    view.roll(options.roll);
    view.yaw(options.yaw);

    let mut measure = Pass::new(false, view.clone());
    measure.run(symbols, parameters);
    let mut paint = Pass::new(true, view);
    paint.run(symbols, parameters);

    Drawing {
        shapes: paint.shapes,
        bounds: Bounds {
            x: measure.min.0,
            y: measure.min.1,
            width: measure.max.0 - measure.min.0,
            height: measure.max.1 - measure.min.1,
        },
    }
}
