use wasmtime::component::*;
use wasmtime::{Engine, Store, Config};
use wasmtime::component::{ResourceTable, Linker};
use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};

use exports::plugnplay::plugnplay::runtime::*;

bindgen!({
    path: "..",
    world: "plugnplay",
});

struct State {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
}

fn main() {
    let mut config = Config::default();
    config.wasm_component_model(true);
    config.debug_info(true);
    let engine = Engine::new(&config).expect("engine");
    let component = Component::from_file(&engine, "../guest/target/wasm32-wasip1/release/guest.wasm").expect("load component");
    let mut linker = Linker::<State>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker).expect("add_to_linker_sync");
    let mut builder = WasiCtxBuilder::new();
    builder.inherit_stdio();
    let mut store = Store::new(
        &engine,
        State {
            ctx: builder.build(),
            table: ResourceTable::new(),
        },
    );

    let bindings = Plugnplay::instantiate(&mut store, &component, &linker).expect("instantiate playground");

    let plugin = bindings.interface0.ctx();
    let ctx = plugin.call_constructor(&mut store).expect("constructor");

    let request = Request::B(format!("function x() {{ return 1 + 42; }}; x()"));
    println!("{:?}", plugin.call_eval(&mut store, ctx, &request));

    let request = Request::B(format!("x() + 9"));
    println!("{:?}", plugin.call_eval(&mut store, ctx, &request));
}

