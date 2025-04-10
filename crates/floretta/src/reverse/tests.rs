use std::{fmt, io::Write};

use goldenfile::Mint;
use rstest::rstest;
use wasmtime::{Engine, Instance, Module, Store, TypedFunc, WasmParams, WasmResults};

use crate::Autodiff;

#[test]
#[cfg(feature = "names")]
fn test_names() {
    let input = wat::parse_str(include_str!("../wat/names.wat")).unwrap();
    let mut ad = Autodiff::new();
    ad.names();
    let output = wasmprinter::print_bytes(ad.reverse(&input).unwrap()).unwrap();
    let mut mint = Mint::new("src/reverse");
    let mut file = mint.new_goldenfile("names.wat").unwrap();
    file.write_all(output.as_bytes()).unwrap();
}

fn compile<P: WasmParams, R: WasmResults, DP: WasmResults, DR: WasmParams>(
    wat: &str,
    name: &str,
) -> (Store<()>, TypedFunc<P, R>, TypedFunc<DR, DP>) {
    let input = wat::parse_str(wat).unwrap();

    let mut ad = Autodiff::new();
    ad.export(name, "backprop");
    let output = ad.reverse(&input).unwrap();

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &output).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let function = instance.get_typed_func::<P, R>(&mut store, name).unwrap();
    let backprop = instance
        .get_typed_func::<DR, DP>(&mut store, "backprop")
        .unwrap();
    (store, function, backprop)
}

struct Backprop<P, R, DP, DR> {
    wat: &'static str,
    name: &'static str,
    input: P,
    output: R,
    cotangent: DR,
    gradient: DP,
}

impl<
        P: fmt::Debug + PartialEq + WasmParams,
        R: fmt::Debug + PartialEq + WasmResults,
        DP: fmt::Debug + PartialEq + WasmResults,
        DR: fmt::Debug + PartialEq + WasmParams,
    > Backprop<P, R, DP, DR>
{
    fn test(self) {
        let (mut store, function, backprop) = compile::<P, R, DP, DR>(self.wat, self.name);
        let output = function.call(&mut store, self.input).unwrap();
        assert_eq!(output, self.output);
        let gradient = backprop.call(&mut store, self.cotangent).unwrap();
        assert_eq!(gradient, self.gradient);
    }
}

#[test]
fn test_square() {
    Backprop {
        wat: include_str!("../wat/square.wat"),
        name: "square",
        input: 3.,
        output: 9.,
        cotangent: 1.,
        gradient: 6.,
    }
    .test()
}

#[test]
fn test_if() {
    let wat = include_str!("../wat/if.wat");
    let (mut store, function, backprop) =
        compile::<(i32, f64, f64), f64, (f64, f64), f64>(wat, "select");
    {
        let output = function.call(&mut store, (1, 2., 3.)).unwrap();
        assert_eq!(output, 2.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (1., 0.));
    }
    {
        let output = function.call(&mut store, (0, 2., 3.)).unwrap();
        assert_eq!(output, 3.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (0., 1.));
    }
}

#[test]
fn test_else() {
    let wat = include_str!("../wat/else.wat");
    let (mut store, function, backprop) =
        compile::<(i32, f64, f64), f64, (f64, f64), f64>(wat, "select");
    {
        let output = function.call(&mut store, (1, 2., 3.)).unwrap();
        assert_eq!(output, 2.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (1., 0.));
    }
    {
        let output = function.call(&mut store, (0, 2., 3.)).unwrap();
        assert_eq!(output, 3.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (0., 1.));
    }
}

#[test]
fn test_br() {
    let wat = include_str!("../wat/br.wat");
    let (mut store, function, backprop) =
        compile::<(i32, f64, f64), f64, (f64, f64), f64>(wat, "select");
    {
        let output = function.call(&mut store, (1, 2., 3.)).unwrap();
        assert_eq!(output, 2.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (1., 0.));
    }
    {
        let output = function.call(&mut store, (0, 2., 3.)).unwrap();
        assert_eq!(output, 3.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (0., 1.));
    }
}

#[test]
fn test_br_if_return() {
    let wat = include_str!("../wat/br_if_return.wat");
    let (mut store, function, backprop) =
        compile::<(i32, f64, f64), f64, (f64, f64), f64>(wat, "select");
    {
        let output = function.call(&mut store, (1, 2., 3.)).unwrap();
        assert_eq!(output, 2.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (1., 0.));
    }
    {
        let output = function.call(&mut store, (0, 2., 3.)).unwrap();
        assert_eq!(output, 3.);
        let gradient = backprop.call(&mut store, 1.).unwrap();
        assert_eq!(gradient, (0., 1.));
    }
}

#[test]
fn test_call() {
    Backprop {
        wat: include_str!("../wat/call.wat"),
        name: "tuple",
        input: (2.0f64, 3i64, 4.0f32, 1i32),
        output: (4.0f32, 1i32, 2.0f64, 3i64),
        cotangent: (5.0f32, 6.0f64),
        gradient: (6.0f64, 5.0f32),
    }
    .test()
}

#[test]
fn test_drop_i32() {
    Backprop {
        wat: include_str!("../wat/drop_i32.wat"),
        name: "drop",
        input: 42,
        output: (),
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_drop_i64() {
    Backprop {
        wat: include_str!("../wat/drop_i64.wat"),
        name: "drop",
        input: 42i64,
        output: (),
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_drop_f32() {
    Backprop {
        wat: include_str!("../wat/drop_f32.wat"),
        name: "drop",
        input: 42.0f32,
        output: (),
        cotangent: (),
        gradient: 0.0f32,
    }
    .test()
}

#[test]
fn test_drop_f64() {
    Backprop {
        wat: include_str!("../wat/drop_f64.wat"),
        name: "drop",
        input: 42.,
        output: (),
        cotangent: (),
        gradient: 0.,
    }
    .test()
}

#[test]
fn test_local_set_f32() {
    Backprop {
        wat: include_str!("../wat/local_set_f32.wat"),
        name: "clobber",
        input: (1.0f32, 2.0f32),
        output: (2.0f32, 2.0f32),
        cotangent: (3.0f32, 4.0f32),
        gradient: (0.0f32, 7.0f32),
    }
    .test()
}

#[test]
fn test_local_set_f64() {
    Backprop {
        wat: include_str!("../wat/local_set_f64.wat"),
        name: "clobber",
        input: (1., 2.),
        output: (2., 2.),
        cotangent: (3., 4.),
        gradient: (0., 7.),
    }
    .test()
}

#[test]
fn test_tuple() {
    Backprop {
        wat: include_str!("../wat/tuple.wat"),
        name: "tuple",
        input: (1i32, 2.0f64, 3i64, 4.0f32),
        output: (4.0f32, 1i32, 2.0f64, 3i64),
        cotangent: (5.0f32, 6.0f64),
        gradient: (6.0f64, 5.0f32),
    }
    .test()
}

#[test]
fn test_loop() {
    Backprop {
        wat: include_str!("../wat/loop.wat"),
        name: "loop",
        input: 1.1,
        output: -0.99,
        cotangent: 1.,
        gradient: 0.20000000000000018,
    }
    .test()
}

#[test]
fn test_f32_store_load() {
    Backprop {
        wat: include_str!("../wat/f32_store_load.wat"),
        name: "roundtrip",
        input: 42.0f32,
        output: 42.0f32,
        cotangent: 1.0f32,
        gradient: 1.0f32,
    }
    .test()
}

#[test]
fn test_f64_store_load() {
    Backprop {
        wat: include_str!("../wat/f64_store_load.wat"),
        name: "roundtrip",
        input: 42.,
        output: 42.,
        cotangent: 1.,
        gradient: 1.,
    }
    .test()
}

#[test]
fn test_i32_const() {
    Backprop {
        wat: include_str!("../wat/i32_const.wat"),
        name: "const",
        input: (),
        output: 42,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_const() {
    Backprop {
        wat: include_str!("../wat/i64_const.wat"),
        name: "const",
        input: (),
        output: 42i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_f32_const() {
    Backprop {
        wat: include_str!("../wat/f32_const.wat"),
        name: "const",
        input: (),
        output: 42.0f32,
        cotangent: 1.0f32,
        gradient: (),
    }
    .test()
}

#[test]
fn test_f64_const() {
    Backprop {
        wat: include_str!("../wat/f64_const.wat"),
        name: "const",
        input: (),
        output: 42.,
        cotangent: 1.,
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_eqz() {
    Backprop {
        wat: include_str!("../wat/i32_eqz.wat"),
        name: "eqz",
        input: 0,
        output: 1,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[rstest]
#[case("i32.eq")]
#[case("i32.ne")]
#[case("i32.lt_s")]
#[case("i32.lt_u")]
#[case("i32.gt_s")]
#[case("i32.gt_u")]
#[case("i32.le_s")]
#[case("i32.le_u")]
#[case("i32.ge_s")]
#[case("i32.ge_u")]
fn i32_logic(#[case] name: &str) {
    compile::<(i32, i32), i32, (), ()>(
        &format!(
            "
(module
  (func (export {name:?}) (param i32 i32) (result i32)
    ({name}
      (local.get 0)
      (local.get 1))))
"
        ),
        name,
    );
}

#[test]
fn test_i64_eqz() {
    Backprop {
        wat: include_str!("../wat/i64_eqz.wat"),
        name: "eqz",
        input: 0i64,
        output: 1,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[rstest]
#[case("i64.eq")]
#[case("i64.ne")]
#[case("i64.lt_s")]
#[case("i64.lt_u")]
#[case("i64.gt_s")]
#[case("i64.gt_u")]
#[case("i64.le_s")]
#[case("i64.le_u")]
#[case("i64.ge_s")]
#[case("i64.ge_u")]
fn i64_logic(#[case] name: &str) {
    compile::<(i64, i64), i32, (), ()>(
        &format!(
            "
(module
  (func (export {name:?}) (param i64 i64) (result i32)
    ({name}
      (local.get 0)
      (local.get 1))))
"
        ),
        name,
    );
}

#[rstest]
#[case("f32.eq")]
#[case("f32.ne")]
#[case("f32.lt")]
#[case("f32.gt")]
#[case("f32.le")]
#[case("f32.ge")]
fn f32_logic(#[case] name: &str) {
    compile::<(f32, f32), i32, (f32, f32), ()>(
        &format!(
            "
(module
  (func (export {name:?}) (param f32 f32) (result i32)
    ({name}
      (local.get 0)
      (local.get 1))))
"
        ),
        name,
    );
}

#[rstest]
#[case("f64.eq")]
#[case("f64.ne")]
#[case("f64.lt")]
#[case("f64.gt")]
#[case("f64.le")]
#[case("f64.ge")]
fn f64_logic(#[case] name: &str) {
    compile::<(f64, f64), i32, (f64, f64), ()>(
        &format!(
            "
(module
  (func (export {name:?}) (param f64 f64) (result i32)
    ({name}
      (local.get 0)
      (local.get 1))))
"
        ),
        name,
    );
}

#[test]
fn test_i32_clz() {
    Backprop {
        wat: include_str!("../wat/i32_clz.wat"),
        name: "clz",
        input: 42,
        output: 26,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_ctz() {
    Backprop {
        wat: include_str!("../wat/i32_ctz.wat"),
        name: "ctz",
        input: 8388608,
        output: 23,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_popcnt() {
    Backprop {
        wat: include_str!("../wat/i32_popcnt.wat"),
        name: "popcnt",
        input: 42,
        output: 3,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_add() {
    Backprop {
        wat: include_str!("../wat/i32_add.wat"),
        name: "add",
        input: (2, 3),
        output: 5,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_sub() {
    Backprop {
        wat: include_str!("../wat/i32_sub.wat"),
        name: "sub",
        input: (5, 3),
        output: 2,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_mul() {
    Backprop {
        wat: include_str!("../wat/i32_mul.wat"),
        name: "mul",
        input: (6, 7),
        output: 42,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_div_s() {
    Backprop {
        wat: include_str!("../wat/i32_div_s.wat"),
        name: "div_s",
        input: (7, 2),
        output: 3,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_div_u() {
    Backprop {
        wat: include_str!("../wat/i32_div_u.wat"),
        name: "div_u",
        input: (7, 2),
        output: 3,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_rem_s() {
    Backprop {
        wat: include_str!("../wat/i32_rem_s.wat"),
        name: "rem_s",
        input: (7, 2),
        output: 1,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_rem_u() {
    Backprop {
        wat: include_str!("../wat/i32_rem_u.wat"),
        name: "rem_u",
        input: (7, 2),
        output: 1,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_and() {
    Backprop {
        wat: include_str!("../wat/i32_and.wat"),
        name: "and",
        input: (3, 2),
        output: 2,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_or() {
    Backprop {
        wat: include_str!("../wat/i32_or.wat"),
        name: "or",
        input: (3, 2),
        output: 3,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_xor() {
    Backprop {
        wat: include_str!("../wat/i32_xor.wat"),
        name: "xor",
        input: (3, 2),
        output: 1,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_shl() {
    Backprop {
        wat: include_str!("../wat/i32_shl.wat"),
        name: "shl",
        input: (1, 2),
        output: 4,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_shr_s() {
    Backprop {
        wat: include_str!("../wat/i32_shr_s.wat"),
        name: "shr_s",
        input: (-4, 1),
        output: -2,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_shr_u() {
    Backprop {
        wat: include_str!("../wat/i32_shr_u.wat"),
        name: "shr_u",
        input: (4, 1),
        output: 2,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_rotl() {
    Backprop {
        wat: include_str!("../wat/i32_rotl.wat"),
        name: "rotl",
        input: (0x12345678, 4),
        output: 0x23456781,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i32_rotr() {
    Backprop {
        wat: include_str!("../wat/i32_rotr.wat"),
        name: "rotr",
        input: (0x12345678, 4),
        output: 0x81234567u32 as i32,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_clz() {
    Backprop {
        wat: include_str!("../wat/i64_clz.wat"),
        name: "clz",
        input: 42i64,
        output: 58i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_ctz() {
    Backprop {
        wat: include_str!("../wat/i64_ctz.wat"),
        name: "ctz",
        input: 8388608i64,
        output: 23i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_popcnt() {
    Backprop {
        wat: include_str!("../wat/i64_popcnt.wat"),
        name: "popcnt",
        input: 42i64,
        output: 3i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_add() {
    Backprop {
        wat: include_str!("../wat/i64_add.wat"),
        name: "add",
        input: (2i64, 3i64),
        output: 5i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_sub() {
    Backprop {
        wat: include_str!("../wat/i64_sub.wat"),
        name: "sub",
        input: (5i64, 3i64),
        output: 2i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_mul() {
    Backprop {
        wat: include_str!("../wat/i64_mul.wat"),
        name: "mul",
        input: (6i64, 7i64),
        output: 42i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_div_s() {
    Backprop {
        wat: include_str!("../wat/i64_div_s.wat"),
        name: "div_s",
        input: (7i64, 2i64),
        output: 3i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_div_u() {
    Backprop {
        wat: include_str!("../wat/i64_div_u.wat"),
        name: "div_u",
        input: (7i64, 2i64),
        output: 3i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_rem_s() {
    Backprop {
        wat: include_str!("../wat/i64_rem_s.wat"),
        name: "rem_s",
        input: (7i64, 2i64),
        output: 1i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_rem_u() {
    Backprop {
        wat: include_str!("../wat/i64_rem_u.wat"),
        name: "rem_u",
        input: (7i64, 2i64),
        output: 1i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_and() {
    Backprop {
        wat: include_str!("../wat/i64_and.wat"),
        name: "and",
        input: (3i64, 2i64),
        output: 2i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_or() {
    Backprop {
        wat: include_str!("../wat/i64_or.wat"),
        name: "or",
        input: (3i64, 2i64),
        output: 3i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_xor() {
    Backprop {
        wat: include_str!("../wat/i64_xor.wat"),
        name: "xor",
        input: (3i64, 2i64),
        output: 1i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_shl() {
    Backprop {
        wat: include_str!("../wat/i64_shl.wat"),
        name: "shl",
        input: (1i64, 2i64),
        output: 4i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_shr_s() {
    Backprop {
        wat: include_str!("../wat/i64_shr_s.wat"),
        name: "shr_s",
        input: (-4i64, 1i64),
        output: -2i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_shr_u() {
    Backprop {
        wat: include_str!("../wat/i64_shr_u.wat"),
        name: "shr_u",
        input: (4i64, 1i64),
        output: 2i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_rotl() {
    Backprop {
        wat: include_str!("../wat/i64_rotl.wat"),
        name: "rotl",
        input: (0x1234567812345678i64, 4i64),
        output: 0x2345678123456781i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_i64_rotr() {
    Backprop {
        wat: include_str!("../wat/i64_rotr.wat"),
        name: "rotr",
        input: (0x1234567812345678i64, 4i64),
        output: 0x8123456781234567u64 as i64,
        cotangent: (),
        gradient: (),
    }
    .test()
}

#[test]
fn test_f32_neg() {
    Backprop {
        wat: include_str!("../wat/f32_neg.wat"),
        name: "neg",
        input: 3.0f32,
        output: -3.0f32,
        cotangent: 1.0f32,
        gradient: -1.0f32,
    }
    .test()
}

#[test]
fn test_f32_sqrt() {
    Backprop {
        wat: include_str!("../wat/f32_sqrt.wat"),
        name: "sqrt",
        input: 16.0f32,
        output: 4.0f32,
        cotangent: 1.0f32,
        gradient: 0.125f32,
    }
    .test()
}

#[test]
fn test_f32_add() {
    Backprop {
        wat: include_str!("../wat/f32_add.wat"),
        name: "add",
        input: (3.0f32, 2.0f32),
        output: 5.0f32,
        cotangent: 1.0f32,
        gradient: (1.0f32, 1.0f32),
    }
    .test()
}

#[test]
fn test_f32_sub() {
    Backprop {
        wat: include_str!("../wat/f32_sub.wat"),
        name: "sub",
        input: (3.0f32, 2.0f32),
        output: 1.0f32,
        cotangent: 1.0f32,
        gradient: (1.0f32, -1.0f32),
    }
    .test()
}

#[test]
fn test_f32_mul() {
    Backprop {
        wat: include_str!("../wat/f32_mul.wat"),
        name: "mul",
        input: (3.0f32, 2.0f32),
        output: 6.0f32,
        cotangent: 1.0f32,
        gradient: (2.0f32, 3.0f32),
    }
    .test()
}

#[test]
fn test_f32_div() {
    Backprop {
        wat: include_str!("../wat/f32_div.wat"),
        name: "div",
        input: (3.0f32, 2.0f32),
        output: 1.5f32,
        cotangent: 1.0f32,
        gradient: (0.5f32, -0.75f32),
    }
    .test()
}

#[test]
fn test_f32_min() {
    Backprop {
        wat: include_str!("../wat/f32_min.wat"),
        name: "min",
        input: (2.0f32, 3.0f32),
        output: 2.0f32,
        cotangent: 1.0f32,
        gradient: (1.0f32, 0.0f32),
    }
    .test()
}

#[test]
fn test_f32_max() {
    Backprop {
        wat: include_str!("../wat/f32_max.wat"),
        name: "max",
        input: (2.0f32, 3.0f32),
        output: 3.0f32,
        cotangent: 1.0f32,
        gradient: (0.0f32, 1.0f32),
    }
    .test()
}

#[test]
fn test_f64_neg() {
    Backprop {
        wat: include_str!("../wat/f64_neg.wat"),
        name: "neg",
        input: 3.,
        output: -3.,
        cotangent: 1.,
        gradient: -1.,
    }
    .test()
}

#[test]
fn test_f64_sqrt() {
    Backprop {
        wat: include_str!("../wat/f64_sqrt.wat"),
        name: "sqrt",
        input: 16.,
        output: 4.,
        cotangent: 1.,
        gradient: 0.125,
    }
    .test()
}

#[test]
fn test_f64_add() {
    Backprop {
        wat: include_str!("../wat/f64_add.wat"),
        name: "add",
        input: (3., 2.),
        output: 5.,
        cotangent: 1.,
        gradient: (1., 1.),
    }
    .test()
}

#[test]
fn test_f64_sub() {
    Backprop {
        wat: include_str!("../wat/f64_sub.wat"),
        name: "sub",
        input: (3., 2.),
        output: 1.,
        cotangent: 1.,
        gradient: (1., -1.),
    }
    .test()
}

#[test]
fn test_f64_mul() {
    Backprop {
        wat: include_str!("../wat/f64_mul.wat"),
        name: "mul",
        input: (3., 2.),
        output: 6.,
        cotangent: 1.,
        gradient: (2., 3.),
    }
    .test()
}

#[test]
fn test_f64_div() {
    Backprop {
        wat: include_str!("../wat/f64_div.wat"),
        name: "div",
        input: (3., 2.),
        output: 1.5,
        cotangent: 1.,
        gradient: (0.5, -0.75),
    }
    .test()
}

#[test]
fn test_f64_min() {
    Backprop {
        wat: include_str!("../wat/f64_min.wat"),
        name: "min",
        input: (2., 3.),
        output: 2.,
        cotangent: 1.,
        gradient: (1., 0.),
    }
    .test()
}

#[test]
fn test_f64_max() {
    Backprop {
        wat: include_str!("../wat/f64_max.wat"),
        name: "max",
        input: (2., 3.),
        output: 3.,
        cotangent: 1.,
        gradient: (0., 1.),
    }
    .test()
}
