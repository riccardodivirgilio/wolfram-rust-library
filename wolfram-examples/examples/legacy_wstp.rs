use wolfram_library_link::{
    self as wll,
    expr::{Expr, ExprKind, Number, Symbol},
    wstp::Link,
};

wll::generate_loader!(load_legacy_wstp_functions);

#[wll::export(wstp)]
fn square_wstp(link: &mut Link) {
    let arg_count: usize = link.test_head("System`List").unwrap();
    if arg_count != 1 {
        panic!("square_wstp: expected 1 argument");
    }
    let x = link.get_i64().expect("expected Integer");
    link.put_i64(x * x).unwrap();
}

#[wll::export(wstp)]
fn count_args(link: &mut Link) {
    let arg_count: usize = link.test_head("System`List").unwrap();
    link.new_packet().unwrap();
    link.put_i64(i64::try_from(arg_count).unwrap()).unwrap();
}

#[wll::export(wstp)]
fn total_args_i64(link: &mut Link) {
    let arg_count: usize = link.test_head("System`List").unwrap();
    let mut total: i64 = 0;
    for _ in 0..arg_count {
        total += link.get_i64().expect("expected Integer");
    }
    link.put_i64(total).unwrap();
}

#[wll::export(wstp)]
fn string_join(link: &mut Link) {
    use wll::wstp::LinkStr;
    let arg_count = link.test_head("System`List").unwrap();
    let mut buf = String::new();
    for _ in 0..arg_count {
        let s: LinkStr<'_> = link.get_string_ref().expect("expected String");
        buf.push_str(s.as_str());
    }
    link.put_str(&buf).unwrap();
}

#[wll::export(wstp)]
fn link_expr_identity(link: &mut Link) {
    let expr = link.get_expr().unwrap();
    link.put_expr(&expr).unwrap();
}

#[wll::export(wstp)]
fn total(args: Vec<Expr>) -> Expr {
    let mut total = Number::Integer(0);
    for (i, arg) in args.into_iter().enumerate() {
        let number = arg
            .try_as_number()
            .unwrap_or_else(|| panic!("expected number at position {}", i + 1));
        use Number::{Integer, Real};
        total = match (total, number) {
            (Integer(a), Integer(b)) => Integer(a + b),
            (Integer(int), Real(real)) | (Real(real), Integer(int)) => {
                Number::real(int as f64 + *real)
            },
            (Real(a), Real(b)) => Real(a + b),
        };
    }
    Expr::number(total)
}

#[wll::export(wstp)]
fn expr_string_join(link: &mut Link) {
    let expr = link.get_expr().unwrap();
    let list = expr.try_as_normal().unwrap();
    assert!(list.has_head(&Symbol::new("System`List")));
    let mut buf = String::new();
    for elem in list.elements() {
        match elem.kind() {
            ExprKind::String(s) => buf.push_str(s),
            _ => panic!("expected String, got: {:?}", elem),
        }
    }
    link.put_str(&buf).unwrap();
}
