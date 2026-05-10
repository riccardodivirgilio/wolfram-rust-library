//! Smoke test for `#[derive(ToWolfram)]` — minimal coverage just to ensure
//! the macro produces compilable code for each shape we support. The full
//! coverage matrix lives in `tests/derive.rs` once the deserialize side
//! also lands.

use wolfram_serializer::{from_wxf, import, to_wxf, Format, FromWolfram, ToWolfram};

#[derive(Debug, PartialEq, ToWolfram, FromWolfram)]
struct Frame {
    payload: Vec<u8>,
    samples: Vec<i32>,
    name: String,
    tag: Option<u32>,
}

#[derive(Debug, PartialEq, ToWolfram, FromWolfram)]
struct Point(f64, f64);

#[derive(Debug, PartialEq, ToWolfram, FromWolfram)]
struct Marker;

#[derive(Debug, PartialEq, ToWolfram, FromWolfram)]
struct Tensor1 {
    fixed: [i32; 4],
    nested: [[f64; 3]; 2],
    tup: (f64, f64, f64),
    nested_tup: ((f64, f64), (f64, f64)),
    hetero: (i64, String),
}

#[derive(Debug, PartialEq, ToWolfram, FromWolfram)]
enum Shape {
    Origin,
    Square(f64),
    Rect(f64, f64),
    Circle { radius: f64 },
}

#[test]
fn frame_roundtrips_with_correct_wire_shapes() {
    let f = Frame {
        payload: vec![1u8, 2, 3, 0xff],
        samples: vec![10i32, 20, 30],
        name: "ada".into(),
        tag: Some(7),
    };
    let bytes = to_wxf(&f).unwrap();
    let expr = import(&bytes, Format::Wxf).unwrap();
    let assoc = expr.try_as_association().expect("Frame should be Association");

    // payload → ByteArray
    let payload = &assoc
        .get(&wolfram_expr::Expr::from("payload"))
        .unwrap()
        .value;
    assert!(payload.try_as_byte_array().is_some(), "payload should be ByteArray");

    // samples → 1-D NumericArray<Integer32>
    let samples = &assoc
        .get(&wolfram_expr::Expr::from("samples"))
        .unwrap()
        .value;
    let na = samples.try_as_numeric_array().expect("samples should be NumericArray");
    assert_eq!(
        na.data_type(),
        wolfram_expr::NumericArrayDataType::Integer32
    );
    assert_eq!(na.dimensions(), &[3]);

    // tag → Integer (since Some)
    let tag = &assoc
        .get(&wolfram_expr::Expr::from("tag"))
        .unwrap()
        .value;
    assert_eq!(tag, &wolfram_expr::Expr::from(7i64));
}

#[test]
fn point_tuple_struct_emits_function() {
    let p = Point(1.5, 2.5);
    let bytes = to_wxf(&p).unwrap();
    let expr = import(&bytes, Format::Wxf).unwrap();
    let normal = expr.try_as_normal().expect("Point should be Function[…]");
    let head = normal.head().try_as_symbol().unwrap().as_str();
    assert_eq!(head, "Global`Point");
    assert_eq!(normal.elements().len(), 2);
}

#[test]
fn marker_unit_struct_emits_symbol() {
    let m = Marker;
    let bytes = to_wxf(&m).unwrap();
    let expr = import(&bytes, Format::Wxf).unwrap();
    let s = expr.try_as_symbol().expect("Marker should be Symbol");
    assert_eq!(s.as_str(), "Global`Marker");
}

#[test]
fn tensor_fields_become_numeric_arrays() {
    let t = Tensor1 {
        fixed: [1, 2, 3, 4],
        nested: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
        tup: (1.0, 2.0, 3.0),
        nested_tup: ((1.0, 2.0), (3.0, 4.0)),
        hetero: (42i64, "hello".into()),
    };
    let bytes = to_wxf(&t).unwrap();
    let expr = import(&bytes, Format::Wxf).unwrap();
    let assoc = expr.try_as_association().unwrap();

    let fixed = &assoc.get(&wolfram_expr::Expr::from("fixed")).unwrap().value;
    let na = fixed.try_as_numeric_array().expect("fixed → NumericArray");
    assert_eq!(na.dimensions(), &[4]);

    let nested = &assoc.get(&wolfram_expr::Expr::from("nested")).unwrap().value;
    let na = nested
        .try_as_numeric_array()
        .expect("nested → 2D NumericArray");
    assert_eq!(na.dimensions(), &[2, 3]);

    let tup = &assoc.get(&wolfram_expr::Expr::from("tup")).unwrap().value;
    let na = tup.try_as_numeric_array().expect("tup → 1D NumericArray");
    assert_eq!(na.dimensions(), &[3]);

    let nested_tup = &assoc
        .get(&wolfram_expr::Expr::from("nested_tup"))
        .unwrap()
        .value;
    let na = nested_tup
        .try_as_numeric_array()
        .expect("nested_tup → 2D NumericArray");
    assert_eq!(na.dimensions(), &[2, 2]);

    // hetero (i64, String) should NOT be a NumericArray; should be a List.
    let hetero = &assoc.get(&wolfram_expr::Expr::from("hetero")).unwrap().value;
    assert!(hetero.try_as_numeric_array().is_none());
    let n = hetero.try_as_normal().expect("hetero → Function[List, …]");
    assert_eq!(n.head().try_as_symbol().unwrap().as_str(), "System`List");
    assert_eq!(n.elements().len(), 2);
}

#[test]
fn frame_roundtrips_through_from_wolfram() {
    let f = Frame {
        payload: vec![1u8, 2, 3, 0xff],
        samples: vec![10i32, 20, 30],
        name: "ada".into(),
        tag: Some(7),
    };
    let bytes = to_wxf(&f).unwrap();
    let back: Frame = from_wxf(&bytes).unwrap();
    assert_eq!(f, back);
}

#[test]
fn frame_with_none_tag_roundtrips() {
    let f = Frame {
        payload: vec![],
        samples: vec![],
        name: "empty".into(),
        tag: None,
    };
    let bytes = to_wxf(&f).unwrap();
    let back: Frame = from_wxf(&bytes).unwrap();
    assert_eq!(f, back);
}

#[test]
fn point_tuple_struct_roundtrips() {
    let p = Point(1.5, 2.5);
    let bytes = to_wxf(&p).unwrap();
    let back: Point = from_wxf(&bytes).unwrap();
    assert_eq!(p, back);
}

#[test]
fn marker_unit_struct_roundtrips() {
    let m = Marker;
    let bytes = to_wxf(&m).unwrap();
    let back: Marker = from_wxf(&bytes).unwrap();
    assert_eq!(m, back);
}

#[test]
fn tensor_struct_roundtrips() {
    let t = Tensor1 {
        fixed: [1, 2, 3, 4],
        nested: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
        tup: (1.0, 2.0, 3.0),
        nested_tup: ((1.0, 2.0), (3.0, 4.0)),
        hetero: (42i64, "hello".into()),
    };
    let bytes = to_wxf(&t).unwrap();
    let back: Tensor1 = from_wxf(&bytes).unwrap();
    assert_eq!(t, back);
}

#[test]
fn enum_roundtrips_all_variant_shapes() {
    for v in [
        Shape::Origin,
        Shape::Square(2.5),
        Shape::Rect(1.0, 2.0),
        Shape::Circle { radius: 3.0 },
    ] {
        let bytes = to_wxf(&v).unwrap();
        let back: Shape = from_wxf(&bytes).unwrap();
        assert_eq!(v, back);
    }
}

#[test]
fn enum_variants_emit_proper_shapes() {
    let bytes = to_wxf(&Shape::Origin).unwrap();
    let s = import(&bytes, Format::Wxf).unwrap();
    assert_eq!(s.try_as_symbol().unwrap().as_str(), "Global`Origin");

    let bytes = to_wxf(&Shape::Square(2.0)).unwrap();
    let s = import(&bytes, Format::Wxf).unwrap();
    let n = s.try_as_normal().unwrap();
    assert_eq!(n.head().try_as_symbol().unwrap().as_str(), "Global`Square");
    assert_eq!(n.elements().len(), 1);

    let bytes = to_wxf(&Shape::Rect(1.0, 2.0)).unwrap();
    let s = import(&bytes, Format::Wxf).unwrap();
    let n = s.try_as_normal().unwrap();
    assert_eq!(n.head().try_as_symbol().unwrap().as_str(), "Global`Rect");
    assert_eq!(n.elements().len(), 2);

    let bytes = to_wxf(&Shape::Circle { radius: 3.0 }).unwrap();
    let s = import(&bytes, Format::Wxf).unwrap();
    let n = s.try_as_normal().unwrap();
    assert_eq!(n.head().try_as_symbol().unwrap().as_str(), "Global`Circle");
    assert_eq!(n.elements().len(), 1);
    let inner = &n.elements()[0];
    let inner_assoc = inner.try_as_association().expect("inner is Association");
    assert!(inner_assoc.get(&wolfram_expr::Expr::from("radius")).is_some());
}
