// Translated from @mapbox/sphericalmercator
// https://github.com/mapbox/sphericalmercator

use std::f64::consts::PI;

const A: f64 = 6378137.0;
const MAXEXTENT: f64 = 20037508.342789244;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct XYZBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct BBox {
    pub w: f64,
    pub s: f64,
    pub e: f64,
    pub n: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LonLatPoint {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct XYPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct SphericalMercator {
    size: u32,
    expansion: f64,
    bc: Vec<f64>,
    cc: Vec<f64>,
    zc: Vec<f64>,
    ac: Vec<f64>,
}

impl SphericalMercator {
    /**
     * Create a new SpphericalMercator with a 256px tile size and antimeridian = false
     */
    pub fn new() -> Self {
        SphericalMercator::new_with_size_and_antimeridian(256, false)
    }

    /**
     * Create a new SphericalMercator with a custom tile size and antimeridian
     */
    pub fn new_with_size_and_antimeridian(size: u32, antimeridian: bool) -> Self {
        let expansion = if antimeridian { 2.0 } else { 1.0 };
        let mut bc = Vec::with_capacity(30);
        let mut cc = Vec::with_capacity(30);
        let mut zc = Vec::with_capacity(30);
        let mut ac = Vec::with_capacity(30);
        let mut size_copy = size as f64;
        for _ in 0..30 {
            bc.push(size_copy / 360.0);
            cc.push(size_copy / (2.0 * PI));
            zc.push(size_copy / 2.0);
            ac.push(size_copy);
            size_copy *= 2.0;
        }
        SphericalMercator {
            size,
            expansion,
            bc,
            cc,
            zc,
            ac,
        }
    }

    fn is_float(n: f64) -> bool {
        n.fract() != 0.0
    }

    /**
     * Convert lon lat to screen pixel value
     */
    pub fn px(&self, ll: LonLatPoint, zoom: f64) -> XYPoint {
        if SphericalMercator::is_float(zoom) {
            let size = self.size as f64 * 2.0_f64.powf(zoom);
            let d = size / 2.0;
            let bc = size / 360.0;
            let cc = size / (2.0 * PI);
            let ac = size;
            let f = ll.lat.to_radians().sin().clamp(-0.9999, 0.9999);
            let mut x = d + ll.lon * bc;
            let mut y = d + 0.5 * ((1.0 + f) / (1.0 - f)).ln() * -cc;
            if x > ac * self.expansion as f64 {
                x = ac * self.expansion as f64;
            }
            if y > ac {
                y = ac;
            }
            XYPoint { x, y }
        } else {
            let zoom = zoom as usize;
            let d = self.zc[zoom];
            let f = ll.lat.to_radians().sin().clamp(-0.9999, 0.9999);
            let mut x = (d + ll.lon * self.bc[zoom]).round();
            let mut y = (d + 0.5 * ((1.0 + f) / (1.0 - f)).ln() * (-self.cc[zoom])).round();
            if x > self.ac[zoom] * self.expansion as f64 {
                x = self.ac[zoom] * self.expansion as f64;
            }
            if y > self.ac[zoom] {
                y = self.ac[zoom];
            }
            XYPoint { x, y }
        }
    }

    /**
     * Convert a screen pixel coordinate to lon lat, at the given zoom level. The inverse of `px`.
     */
    pub fn ll(&self, px: XYPoint, zoom: f64) -> LonLatPoint {
        if SphericalMercator::is_float(zoom) {
            let size = self.size as f64 * 2.0_f64.powf(zoom);
            let bc = size / 360.0;
            let cc = size / (2.0 * PI);
            let zc = size / 2.0;
            let g = (px.y - zc) / -cc;
            let lon = (px.x - zc) / bc;
            let lat = (2.0 * g.exp().atan() - 0.5 * PI).to_degrees();
            LonLatPoint { lon, lat }
        } else {
            let zoom = zoom as usize;
            let g = (px.y - self.zc[zoom]) / -self.cc[zoom];
            let lon = (px.x - self.zc[zoom]) / self.bc[zoom];
            let lat = (2.0 * g.exp().atan() - 0.5 * PI).to_degrees();
            LonLatPoint { lon, lat }
        }
    }

    /**
     * Convert the tile xyz to a bounding box.
     */
    pub fn bbox(&self, x: u32, y: u32, zoom: u32, tms_style: bool, srs: &str) -> BBox {
        let mut y = y;
        if tms_style {
            y = (2_u32.pow(zoom as u32) - 1) - y;
        }
        let ll = XYPoint {
            x: x as f64 * self.size as f64,
            y: (y as f64 + 1.0) * self.size as f64,
        };
        let ur = XYPoint {
            x: (x as f64 + 1.0) * self.size as f64,
            y: y as f64 * self.size as f64,
        };
        let ll_ll = self.ll(ll, zoom as f64);
        let ur_ll = self.ll(ur, zoom as f64);
        let bbox = BBox {
            w: ll_ll.lon,
            s: ll_ll.lat,
            e: ur_ll.lon,
            n: ur_ll.lat,
        };
        if srs == "900913" {
            self.convert(bbox, "900913")
        } else {
            bbox
        }
    }

    /**
     * Convert a latitude/longitude bbox to xyx bounds.
     * If `tms_style` is true, y values are flipped.
     * If `srs` is "900913", bbox is converted to WGS84.
     */
    pub fn xyz(&self, bbox: BBox, zoom: u32, tms_style: bool, srs: &str) -> XYZBounds {
        let bbox = if srs == "900913" {
            self.convert(bbox, "WGS84")
        } else {
            bbox
        };
        let ll = LonLatPoint {
            lon: bbox.w,
            lat: bbox.s,
        };
        let ur = LonLatPoint {
            lon: bbox.e,
            lat: bbox.n,
        };
        let px_ll = self.px(ll, zoom as f64);
        let px_ur = self.px(ur, zoom as f64);

        let size = self.size as f64;
        let x0 = (px_ll.x / size).floor() as u32;
        let x1 = ((px_ur.x - 1.0_f64) / size).floor() as u32;
        let y0 = (px_ur.y / size).floor() as u32;
        let y1 = ((px_ll.y - 1.0_f64) / size).floor() as u32;

        let mut bounds = XYZBounds {
            min_x: x0.min(x1),
            min_y: y0.min(y1),
            max_x: x0.max(x1),
            max_y: y0.max(y1),
        };
        if tms_style {
            let tms_min_y = (2_u32.pow(zoom as u32) - 1) - bounds.max_y;
            let tms_max_y = (2_u32.pow(zoom as u32) - 1) - bounds.min_y;
            bounds.min_y = tms_min_y;
            bounds.max_y = tms_max_y;
        }
        bounds
    }

    /**
     * Convert a bbox from one projection to another (default WGS84 to 900913)
     */
    pub fn convert(&self, bbox: BBox, to: &str) -> BBox {
        if to == "900913" {
            let ll_forward = self.forward(LonLatPoint {
                lon: bbox.w,
                lat: bbox.s,
            });
            let ur_forward = self.forward(LonLatPoint {
                lon: bbox.e,
                lat: bbox.n,
            });
            BBox {
                w: ll_forward.x,
                s: ll_forward.y,
                e: ur_forward.x,
                n: ur_forward.y,
            }
        } else {
            let ll_inverse = self.inverse(XYPoint {
                x: bbox.w,
                y: bbox.s,
            });
            let ur_inverse = self.inverse(XYPoint {
                x: bbox.e,
                y: bbox.n,
            });
            BBox {
                w: ll_inverse.lon,
                s: ll_inverse.lat,
                e: ur_inverse.lon,
                n: ur_inverse.lat,
            }
        }
    }

    /**
     * Convert lon, lat values to mercator x, y
     */
    pub fn forward(&self, ll: LonLatPoint) -> XYPoint {
        let mut xy = XYPoint {
            x: (A * ll.lon).to_radians(),
            y: A * ((PI * 0.25) + ((0.5 * ll.lat).to_radians())).tan().ln(),
        };
        if xy.x > MAXEXTENT {
            xy.x = MAXEXTENT;
        }
        if xy.x < -MAXEXTENT {
            xy.x = -MAXEXTENT;
        }
        if xy.y > MAXEXTENT {
            xy.y = MAXEXTENT;
        }
        if xy.y < -MAXEXTENT {
            xy.y = -MAXEXTENT;
        }
        xy
    }

    /**
     * Convert mercator x, y values to lon, lat
     */
    pub fn inverse(&self, xy: XYPoint) -> LonLatPoint {
        LonLatPoint {
            lon: (xy.x).to_degrees() / A,
            lat: (PI * 0.5 - 2.0 * (-xy.y / A).exp().atan()).to_degrees(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    const MAX_EXTENT_MERC: BBox = BBox {
        w: -20037508.342789244,
        s: -20037508.342789244,
        e: 20037508.342789244,
        n: 20037508.342789244,
    };
    const MAX_EXTENT_WGS84: BBox = BBox {
        w: -180.0,
        s: -85.0511287798066,
        e: 180.0,
        n: 85.0511287798066,
    };

    #[test]
    fn test_bbox() {
        let sm = SphericalMercator::new();
        assert_eq!(
            sm.bbox(0, 0, 0, true, "WGS84"),
            BBox {
                w: -180.0,
                s: -85.05112877980659,
                e: 180.0,
                n: 85.0511287798066
            }
        );
        assert_eq!(
            sm.bbox(0, 0, 1, true, "WGS84"),
            BBox {
                w: -180.0,
                s: -85.05112877980659,
                e: 0.0,
                n: 0.0
            }
        );
    }

    #[test]
    fn test_xyz() {
        let sm = SphericalMercator::new();
        assert_eq!(
            sm.xyz(
                BBox {
                    w: -180.0,
                    s: -85.05112877980659,
                    e: 180.0,
                    n: 85.0511287798066
                },
                0,
                true,
                "WGS84"
            ),
            XYZBounds {
                min_x: 0,
                min_y: 0,
                max_x: 0,
                max_y: 0,
            }
        );
        assert_eq!(
            sm.xyz(
                BBox {
                    w: -180.0,
                    s: -85.05112877980659,
                    e: 0.0,
                    n: 0.0
                },
                1,
                true,
                "WGS84"
            ),
            XYZBounds {
                min_x: 0,
                min_y: 0,
                max_x: 0,
                max_y: 0,
            }
        );
    }

    #[test]
    fn test_xyz_broken() {
        let sm = SphericalMercator::new();
        let extent = BBox {
            w: -0.087891,
            s: 40.95703,
            e: 0.087891,
            n: 41.044916,
        };
        let xyz = sm.xyz(extent.clone(), 3, true, "WGS84");
        assert!(
            xyz.min_x <= xyz.max_x,
            "x: {} <= {} for {:?}",
            xyz.min_x,
            xyz.max_x,
            extent
        );
        assert!(
            xyz.min_y <= xyz.max_y,
            "y: {} <= {} for {:?}",
            xyz.min_y,
            xyz.max_y,
            extent
        );
    }

    #[test]
    fn test_xyz_negative() {
        let sm = SphericalMercator::new();
        let extent = BBox {
            w: -112.5,
            s: 85.0511,
            e: -112.5,
            n: 85.0511,
        };
        let xyz = sm.xyz(extent, 0, false, "WGS84");
        assert_eq!(xyz.min_y, 0);
    }

    #[test]
    fn test_xyz_fuzz() {
        let sm = SphericalMercator::new();
        for _ in 0..1000 {
            let x = [
                -180.0 + (360.0 * random::<f64>()),
                -180.0 + (360.0 * random::<f64>()),
            ];
            let y = [
                -85.0 + (170.0 * random::<f64>()),
                -85.0 + (170.0 * random::<f64>()),
            ];
            let z = (22.0 * random::<f64>()).floor() as u32;
            let extent = BBox {
                w: x[0].min(x[1]),
                s: y[0].min(y[1]),
                e: x[0].max(x[1]),
                n: y[0].max(y[1]),
            };
            let xyz = sm.xyz(extent.clone(), z, true, "WGS84");
            if xyz.min_x > xyz.max_x {
                assert!(
                    xyz.min_x <= xyz.max_x,
                    "x: {} <= {} for {:?}",
                    xyz.min_x,
                    xyz.max_x,
                    extent
                );
            }
            if xyz.min_y > xyz.max_y {
                assert!(
                    xyz.min_y <= xyz.max_y,
                    "y: {} <= {} for {:?}",
                    xyz.min_y,
                    xyz.max_y,
                    extent
                );
            }
        }
    }

    #[test]
    fn test_convert() {
        let sm = SphericalMercator::new();
        assert_eq!(sm.convert(MAX_EXTENT_WGS84, "900913"), MAX_EXTENT_MERC);
        assert_eq!(sm.convert(MAX_EXTENT_MERC, "WGS84"), MAX_EXTENT_WGS84);
    }

    #[test]
    fn test_extents() {
        let sm = SphericalMercator::new();
        assert_eq!(
            sm.convert(
                BBox {
                    w: -240.0,
                    s: -90.0,
                    e: 240.0,
                    n: 90.0
                },
                "900913"
            ),
            MAX_EXTENT_MERC
        );
        assert_eq!(
            sm.xyz(
                BBox {
                    w: -240.0,
                    s: -90.0,
                    e: 240.0,
                    n: 90.0
                },
                4,
                true,
                "WGS84"
            ),
            XYZBounds {
                min_x: 0,
                min_y: 0,
                max_x: 15,
                max_y: 15,
            }
        );
    }

    #[test]
    fn test_ll() {
        let sm = SphericalMercator::new();
        assert_eq!(
            sm.ll(XYPoint { x: 200.0, y: 200.0 }, 9.0),
            LonLatPoint {
                lon: -179.45068359375,
                lat: 85.00351401304403
            }
        );
        assert_eq!(
            sm.ll(XYPoint { x: 200.0, y: 200.0 }, 8.6574),
            LonLatPoint {
                lon: -179.3034449476476,
                lat: 84.99067388699072,
            }
        );
    }

    #[test]
    fn test_px() {
        let sm = SphericalMercator::new();
        let anti_m = SphericalMercator::new_with_size_and_antimeridian(256, true);
        assert_eq!(
            sm.px(
                LonLatPoint {
                    lon: -179.0,
                    lat: 85.0
                },
                9.0
            ),
            XYPoint { x: 364.0, y: 215.0 }
        );
        assert_eq!(
            sm.px(
                LonLatPoint {
                    lon: -179.0,
                    lat: 85.0
                },
                8.6574
            ),
            XYPoint {
                x: 287.12734093961626,
                y: 169.30444219392666
            }
        );
        assert_eq!(
            sm.px(
                LonLatPoint {
                    lon: 250.0,
                    lat: 3.0
                },
                4.0
            ),
            XYPoint {
                x: 4096.0,
                y: 2014.0
            }
        );
        assert_eq!(
            anti_m.px(
                LonLatPoint {
                    lon: 250.0,
                    lat: 3.0
                },
                4.0
            ),
            XYPoint {
                x: 4892.0,
                y: 2014.0
            }
        );
        assert_eq!(
            anti_m.px(
                LonLatPoint {
                    lon: 400.0,
                    lat: 3.0
                },
                4.0
            ),
            XYPoint {
                x: 6599.0,
                y: 2014.0
            }
        );
    }

    #[test]
    fn test_high_precision_float() {
        let sm = SphericalMercator::new();
        let with_int = sm.ll(XYPoint { x: 200.0, y: 200.0 }, 4.0);
        let with_float = sm.ll(XYPoint { x: 200.0, y: 200.0 }, 4.0000000001);

        fn round(val: f64) -> String {
            format!("{:.6}", val)
        }

        assert_eq!(round(with_int.lon), round(with_float.lon));
        assert_eq!(round(with_int.lat), round(with_float.lat));
    }
}
