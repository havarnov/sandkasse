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


pub enum Value {
    Void,
    Int(i32),
    Bool(bool),
    Str(String),
}

pub trait FromJs : Sized {
    fn from_js(value: Value) -> Result<Self, Error>;
    fn response_type() -> ResponseType;
}

impl FromJs for () {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Void => Ok(()),
            _ => Err(Error::WrongType(format!("expected void"))),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Void
    }
}

impl FromJs for bool {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Bool(v) => Ok(v),
            _ => Err(Error::WrongType(format!("expected bool"))),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Boolean
    }
}

impl FromJs for i32 {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Int(int) => Ok(int),
            _ => Err(Error::WrongType(format!("expected int"))),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Int
    }
}

impl FromJs for String {
    fn from_js(value: Value) -> Result<Self, Error> {
        match value {
            Value::Str(v) => Ok(v),
            _ => Err(Error::WrongType(format!("expected string"))),
        }
    }

    fn response_type() -> ResponseType {
        ResponseType::Str
    }
}

impl<'a> Context<'a> {
    pub fn eval<V: FromJs>(&mut self, source: String) -> Result<V, Error> {
        let request = EvalParams { source, response_type: V::response_type(), };
        let response = self.ctx.call_eval(&mut self.store, self.resource, &request)?;
        match response {
            Ok(Response::Void) => V::from_js(Value::Void),
            Ok(Response::Int(v)) => V::from_js(Value::Int(v)),
            Ok(Response::Boolean(v)) => V::from_js(Value::Bool(v)),
            Ok(Response::Str(v)) => V::from_js(Value::Str(v)),
            Err(e) => Err(Error::WrongType(format!("{:?}", e))),
        }
    }

    pub fn register(&mut self, name: String, is_int: bool) -> Result<(), Error> {
        let request = RegisterParams { name, is_int, };
        let _response = self.ctx.call_register(&mut self.store,self.resource, &request)?;
        Ok(())
    }
}

