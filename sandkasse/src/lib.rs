use wasmtime::component::*;
use wasmtime::{Engine, Store, Config};
use wasmtime::component::{ResourceTable, Linker};
use wasmtime_wasi::{IoView, WasiCtx, WasiView, WasiCtxBuilder};

use exports::havarnov::sandkasse::runtime::*;

bindgen!({
    path: "..",
    world: "sandkasse",
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

#[derive(Debug)]
pub enum Error {
    Init(String),
    WrongType(String),
}

impl From<wasmtime::Error> for Error {
    fn from(error: wasmtime::Error) -> Self {
        Self::Init(format!("{:?}", error))
    }
}

pub struct Runtime {
    store: Store<State>,
    package: Sandkasse,
}

pub struct Context<'a> {
    store: &'a mut Store<State>,
    ctx: GuestCtx<'a>,
    resource: ResourceAny,
}

impl Runtime {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::default();
        config.wasm_component_model(true);

        // TODO: configuration
        // config.debug_info(true);
        // config.consume_fuel(true);

        let engine = Engine::new(&config)?;

        // TODO: load from binary
        let component = Component::from_file(&engine, "../guest/target/wasm32-wasip1/release/guest.wasm")?;

        let mut linker = Linker::<State>::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;

        let mut builder = WasiCtxBuilder::new();

        // TODO: config
        builder.inherit_stdio();

        let mut store = Store::new(
            &engine,
            State {
                ctx: builder.build(),
                table: ResourceTable::new(),
            },
        );
        // store.set_fuel(4_000_000).expect("set fuel");

        let package = Sandkasse::instantiate(&mut store, &component, &linker)?;

        Ok(Runtime { store, package, })
    }

    pub fn create_ctx<'a>(&'a mut self) -> Result<Context<'a>, Error> {
        let ctx = self.package.interface0.ctx();
        let resource = ctx.call_constructor(&mut self.store)?;
        Ok(Context { store: &mut self.store, ctx: ctx, resource, })
    }
}


enum Value {
    Void,
    Int(i32),
}

pub trait FromJs : Sized {
    fn from_js(value: Value) -> Result<Self, Error>;
}

impl FromJs for () {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Void => Ok(()),
            _ => Err(Error::WrongType(format!("expected void"))),
        }
    }
}

impl FromJs for i32 {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Int(int) => Ok(int),
            _ => Err(Error::WrongType(format!("expected int"))),
        }
    }
}

impl<'a> Context<'a> {
    pub fn eval<V: FromJs>(&mut self, script: String) -> Result<V, Error> {
        let request = Request::Eval(script);
        let response = self.ctx.call_handle(&mut self.store, self.resource, &request)?;
        match response {
            Ok(Response::Int(int)) => V::from_js(Value::Int(int)),
            _ => todo!("todo")
        }
    }

    pub fn register(&mut self, name: String, is_int: bool) -> Result<(), Error> {
        let request = RegisterParams { name, is_int, };
        let _response = self.ctx.call_register(&mut self.store,self.resource, &request)?;
        Ok(())
    }
}

