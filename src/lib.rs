use std::thread;

use libmem::*;
use mlua::{Error, LightUserData, MultiValue, Value};
use mlua::prelude::*;
use rustyline::DefaultEditor;
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

fn init_repl() {
    LUASTATE.with(|lua| {
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
                if let Some(_module) = find_module(module.as_str()) {
                    if let Some(_addr) = unsafe { sig_scan(sig.as_str(), _module.base, _module.size) } {
                        return Ok(MultiValue::from_iter(vec![Value::LightUserData(LightUserData(_addr as *mut _))]));
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
    });
}
