//! Offline renders of the actual host overlay. No Android or vehicle connection.
//! cargo run --release --manifest-path sim/Cargo.toml --example unlock-preview -- host/out/unlock

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter, WindowEvent, PointerEventButton};
use slint::ComponentHandle;
use slint_interpreter::{Compiler, Value};
use std::{cell::{Cell, RefCell}, error::Error, path::PathBuf, rc::Rc, time::Duration};

struct Headless {
    window: Rc<MinimalSoftwareWindow>,
    clock: Rc<Cell<Duration>>,
}
impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> Duration { self.clock.get() }
}

fn main() -> Result<(), Box<dyn Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let out = std::env::args().nth(1).map(PathBuf::from).unwrap_or(repo.join("host/out/unlock"));
    std::fs::create_dir_all(&out)?;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let clock = Rc::new(Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(Headless { window: window.clone(), clock: clock.clone() }))?;
    let source = format!(r#"
        import {{ Dash }} from "{}";
        import {{ HostOverlay, HostState }} from "{}";
        import {{ Vehicle, Media, Actions }} from "vehicle.slint";
        export {{ Vehicle, Media, Actions, HostState }}
        export component Preview inherits Window {{
            callback unlock-key(int);
            width: 1280px; height: 768px;
            default-font-family: theme.default-font-family;
            theme := Dash {{ width: parent.width; height: parent.height; }}
            HostOverlay {{
                width: parent.width; height: parent.height;
                unlock-key(key) => {{ root.unlock-key(key); }}
            }}
        }}
    "#, repo.join("themes/default/main.slint").display(), repo.join("host/ui/overlay.slint").display());
    let mut compiler = Compiler::default();
    compiler.set_include_paths(vec![repo.join("contract")]);
    let result = pollster::block_on(compiler.build_from_source(source, repo.join("host/ui/preview.slint")));
    for diagnostic in result.diagnostics() { eprintln!("{diagnostic}"); }
    if result.has_errors() { return Err("host overlay did not compile".into()); }
    let definition = result.component("Preview").ok_or("Preview component missing")?;
    let scenarios = [
        ("connecting", 0, 0, -1, 0, 0, false, 0),
        ("startup-service-error", 0, 0, -1, 2, 0, false, 0),
        ("keypad", 1, 3, 5, 0, 0, true, 3),
        ("wrong-pin", 1, 0, 4, 1, 0, true, 0),
        ("service-error", 1, 0, -1, 2, 0, false, 0),
        ("lockout", 1, 0, 0, 1, 127, false, 0),
        ("verifying", 2, 0, 5, 0, 0, false, 0),
        ("unlocking", 3, 0, 5, 0, 0, false, 0),
        ("parked", 4, 0, 5, 0, 0, false, 3),
        ("driving-normal", 5, 0, 5, 0, 0, false, 0),
    ];
    for (index, (name, phase, digits, attempts, error, cooldown, available, alert)) in scenarios.iter().enumerate() {
        let ui = definition.create()?;
        let keys = Rc::new(RefCell::new(Vec::new()));
        let captured = keys.clone();
        ui.set_callback("unlock-key", move |args| {
            if let Some(Value::Number(key)) = args.first() { captured.borrow_mut().push(*key as i32); }
            Value::Void
        })?;
        for (property, value) in [
            ("auth-phase", *phase), ("pin-length", *digits), ("auth-attempts", *attempts),
            ("auth-error", *error), ("auth-cooldown", *cooldown), ("alert-code", *alert),
        ] { ui.set_global_property("HostState", property, Value::Number(value as f64))?; }
        ui.set_global_property("HostState", "auth-available", Value::Bool(*available))?;
        ui.set_global_property("HostState", "side-stand", Value::Bool(*alert == 3))?;
        // The overlay reads the HostState copies; the theme behind it reads Vehicle.
        ui.set_global_property("Vehicle", "locked", Value::Bool(*phase < 3))?;
        ui.set_global_property("HostState", "locked", Value::Bool(*phase < 3))?;
        ui.set_global_property("Vehicle", "ready-to-ride", Value::Bool(*phase == 5))?;
        ui.set_global_property("HostState", "ready", Value::Bool(*phase == 5))?;
        ui.set_global_property("Vehicle", "battery-pct", Value::Number(70.0))?;
        ui.set_global_property("HostState", "battery-pct", Value::Number(70.0))?;
        ui.set_global_property("Vehicle", "range-km", Value::Number(80.0))?;
        ui.set_global_property("Vehicle", "clock-text", Value::String("10:24".into()))?;
        ui.set_global_property("HostState", "clock-text", Value::String("10:24".into()))?;
        ui.set_global_property("Vehicle", "mode", Value::EnumerationValue("DriveMode".into(),
            if *phase == 5 { "normal" } else { "park" }.into()))?;
        ui.show()?;
        window.set_size(slint::PhysicalSize::new(1280, 768));
        let mut buffer = vec![PremultipliedRgbaColor::default(); 1280 * 768];
        for frame in 0..60 {
            clock.set(Duration::from_millis((index * 1000 + frame * 16) as u64));
            slint::platform::update_timers_and_animations();
            window.request_redraw();
            window.draw_if_needed(|renderer| { renderer.render(&mut buffer, 1280); });
        }
        let pixels: Vec<u8> = buffer.iter().flat_map(|p| [p.red, p.green, p.blue, p.alpha]).collect();
        let file = std::fs::File::create(out.join(format!("{name}.png")))?;
        let mut encoder = png::Encoder::new(file, 1280, 768);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&pixels)?;
        // Actual pointer dispatch through the shipped host overlay, never an IPC call.
        for (x, y) in [(674.0, 304.0), (1082.0, 640.0), (674.0, 640.0)] {
            let position = slint::LogicalPosition::new(x, y);
            ui.window().dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
            ui.window().dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
        }
        if *available {
            assert_eq!(*keys.borrow(), vec![1, 10, 11], "keypad digit/delete/clear dispatch");
        } else {
            assert!(keys.borrow().is_empty(), "disabled/hidden keypad dispatched a key in {name}");
        }
        ui.hide()?;
        println!("rendered {name}");
    }
    Ok(())
}
