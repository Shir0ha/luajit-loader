use std::str::FromStr;
use std::thread;

use mlua::{Error, LightUserData, MultiValue, Value};
use mlua::prelude::*;
use rustyline::DefaultEditor;
use skidscan::*;
use windows::{Win32::Foundation::*, Win32::System::Console::*, Win32::System::SystemServices::*};

thread_local! {
    static LUASTATE: Lua = unsafe { Lua::unsafe_new() }
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            unsafe { AllocConsole() }.expect("alloc console failed");
            thread::spawn(init_repl);
        }
        DLL_PROCESS_DETACH => {
            drop(LUASTATE);
            unsafe { FreeConsole() }.expect("free console failed");
        }
        _ => (),
    }

    true
}

fn format_val(values: MultiValue) -> String {
    return values.iter().map(|v| format!("{:#?}", v)).collect::<Vec<_>>().join("    ");
}

fn init_globals(lua: &Lua) -> Result<(), Error> {
    let globals = lua.globals();
    globals.set(
        "print",
        lua.create_function(|_, values: MultiValue| {
            println!(
                "< {}",
                format_val(values)
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "error",
        lua.create_function(|_, values: MultiValue| {
            eprintln!(
                "! < {}",
                format_val(values)
            );
            Ok(())
        })?,
    )?;
    globals.set(
        "find_pattern",
        lua.create_function(|_, (module, sig): (String, String)| {
            if let Ok(_sig) = Signature::from_str(sig.as_str()) {
                if let Ok(_addr) = unsafe { _sig.scan_module(module.as_str()) } {
                    return Ok(MultiValue::from_iter(vec![Value::LightUserData(LightUserData(_addr as *mut _))]));
                }
            }

            return Ok(MultiValue::new());
        })?,
    )?;
    Ok(())
}

fn init_repl() {
    LUASTATE.with(|lua| {
        if let Err(e) = init_globals(lua) {
            eprintln!("{}", e);
        }
        
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
    });
}
