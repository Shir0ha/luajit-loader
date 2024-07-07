use mlua::{Error, LightUserData, MultiValue, Value};
use mlua::prelude::*;
use proc_mem::{Process, Signature};
use rustyline::DefaultEditor;
use windows::{Win32::Foundation::*, Win32::System::Console::*, Win32::System::SystemServices::*};

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: u32, _: *mut ()) -> bool {
    unsafe {
        let lua = Lua::unsafe_new();
        match call_reason {
            DLL_PROCESS_ATTACH => attach(&lua),
            DLL_PROCESS_DETACH => drop(lua),
            _ => (),
        }
    }

    true
}

fn format_val(values: MultiValue) -> String {
    return values.iter().map(|v| format!("{:#?}", v)).collect::<Vec<_>>().join("    ")
}

unsafe fn attach(lua: &Lua) {
    AllocConsole().expect("alloc console failed");
    let globals = lua.globals();
    globals.set(
        "print",
        lua.create_function(|_, values: MultiValue| {
            println!(
                "< {}",
                format_val(values)
            );
            Ok(())
        }).expect("print failed"),
    ).expect("set print failed");
    globals.set(
        "error",
        lua.create_function(|_, values: MultiValue| {
            eprintln!(
                "! < {}",
                format_val(values)
            );
            Ok(())
        }).expect("error failed"),
    ).expect("set error failed");
    globals.set(
        "find_pattern",
        lua.create_function(|_, (module, sig): (String, String)| {
            if let Ok(process) = Process::with_pid(std::process::id()) {
                if let Ok(_module) = process.module(module.as_str()) {
                    let _sig = Signature {
                        name: "".to_string(),
                        pattern: sig,
                        offsets: vec![],
                        extra: 0,
                        relative: false,
                        rip_relative: false,
                        rip_offset: 0,
                    };
                    if let Ok(_addr) = _module.find_signature(&_sig) {
                        return Ok(MultiValue::from_iter(vec![Value::LightUserData(LightUserData(_addr as *mut _))]));
                    }
                }
            }

            return Ok(MultiValue::new());
        }).expect("find_pattern failed"),
    ).expect("set find_pattern failed");

    let mut editor = DefaultEditor::new().expect("Failed to create editor");
    loop {
        let mut prompt = "> ";
        let mut line = String::new();

        loop {
            match editor.readline(prompt) {
                Ok(input) => line.push_str(&input),
                Err(_) => return,
            }

            match lua.load(&line).set_name("").eval::<MultiValue>() {
                Ok(values) => {
                    editor.add_history_entry(line).unwrap();
                    println!(
                        "<·{}",
                        format_val(values)
                    );
                    break;
                }
                Err(Error::SyntaxError {
                        incomplete_input: true,
                        ..
                    }) => {
                    // continue reading input and append it to `line`
                    line.push_str("\n"); // separate input lines
                    prompt = ">> ";
                }
                Err(e) => {
                    eprintln!("{}", e.into_lua_err());
                    break;
                }
            }
        }
    }
}
