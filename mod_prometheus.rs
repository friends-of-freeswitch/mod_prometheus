#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate freeswitchrs;
extern crate prometheus;

use std::sync::{Arc, Mutex};

use freeswitchrs::raw as fsr;
use freeswitchrs::mods::*; // This will get replaced with a mods prelude
use freeswitchrs::Status;

use prometheus::Registry;

lazy_static! {
    static ref REGISTRY: Arc<Mutex<Registry>> = {
        Arc::new(Mutex::new(prometheus::Registry::new("0.0.0.0".to_string(), 6780)))
    };
}

fn prometheus_load(mod_int: &ModInterface) -> Status {
    mod_int.add_raw_api("counter_increase", "Increase counter", "counter_increase", counter_increase_api);

    // Example of binding to an event
    /*
    freeswitchrs::event_bind("asd", fsr::event_types::ALL, None, |e| {
        let s = e.subclass_name();
        let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
        println!("{:?}/{:?} {} = {:?}", e.event_id(), s, e.flags(), b)
    });
    */
    let ref reg = *REGISTRY;
    prometheus::Registry::start(&reg);
    Ok(())
}

fn prometheus_unload() -> Status {
    let ref reg = *REGISTRY;
    prometheus::Registry::stop(&reg);
    Ok(())
}

static MOD_PROMETHEUS_DEF: ModDefinition = ModDefinition {
    name: "mod_prometheus",
    load: prometheus_load,
    shutdown: Some(prometheus_unload),
    runtime: None,
};

freeswitch_export_mod!(libmod_prometheus_module_interface, MOD_PROMETHEUS_DEF);

#[allow(unused_variables)]
unsafe extern "C" fn counter_increase_api(cmd: *const std::os::raw::c_char,
                                          session: *mut fsr::core_session,
                                          stream: *mut fsr::stream_handle)
                                          -> fsr::status {
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr("OK"));
    fsr::status::SUCCESS
}
