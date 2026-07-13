//! Native desktop app for labeling recipe pages.

use dioxus::prelude::*;
use gliphoread::model::{PageAnnotation, Role, WordAnnotation};
use image::GenericImageView;
use rfd::FileDialog;
use std::path::{Path, PathBuf};

fn find_annotations(base_dir: &Path) -> Vec<PathBuf> {
    let mut annots = Vec::new();
    let pages_dir = base_dir.join("scansioni documenti pastry/pages_png");
    if let Ok(entries) = std::fs::read_dir(&pages_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "annot.json") {
                annots.push(path);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "annot.json") {
                annots.push(path);
            }
        }
    }
    annots
}

fn load_annotation(path: &Path) -> Option<PageAnnotation> {
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

fn load_data() -> AppState {
    let base = std::env::current_dir().unwrap_or_default();
    let mut annotations: Vec<PageAnnotation> = Vec::new();
    let mut image_paths: Vec<String> = Vec::new();

    for annot_path in find_annotations(&base) {
        if let Some(annot) = load_annotation(&annot_path) {
            let img_path = if !annot.image_path.is_empty() && Path::new(&annot.image_path).exists() {
                if let Ok(img) = image::open(&annot.image_path) {
                    let thumb = img.thumbnail(1200, 1300);
                    let out_dir = base.join("label_cache");
                    let _ = std::fs::create_dir_all(&out_dir);
                    let out_path = out_dir.join(format!("{}_thumb.png", annot.page.replace(".png", "")));
                    if thumb.save(&out_path).is_ok() {
                        out_path.to_string_lossy().to_string()
                    } else {
                        annot.image_path.clone()
                    }
                } else {
                    annot.image_path.clone()
                }
            } else {
                annot.image_path.clone()
            };
            image_paths.push(img_path);
            annotations.push(annot);
        }
    }

    if annotations.is_empty() {
        annotations.push(PageAnnotation {
            page: "demo".to_string(),
            image_path: "".to_string(),
            width: 800,
            height: 900,
            words: Vec::new(),
        });
        image_paths.push(String::new());
    }

    AppState { annotations, image_paths }
}

#[derive(Clone)]
struct AppState {
    annotations: Vec<PageAnnotation>,
    image_paths: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum DrawMode {
    #[default]
    None,
    Drawing,
}

#[component]
fn word_edit(word_signal: Signal<WordAnnotation>, idx: usize) -> Element {
    let roles = [
        Role::Unlabeled, Role::Title, Role::Ingredient,
        Role::Measurement, Role::Unit, Role::Instruction,
        Role::ListItem, Role::Comment,
    ];

    rsx! {
        div {
            class: "word-card",
            onclick: move |_| { word_signal.write().list_pos = Some(idx as u8); },
            div { class: "word-header",
                span { class: "word-text", "{word_signal.read().text}" }
                span { class: "word-conf", "conf: {word_signal.read().confidence:.0}%" }
                div {
                    class: "role-indicator",
                    style: format!("border-left: 3px solid {}", word_signal.read().role.color()),
                }
                button {
                    class: "delete-btn",
                    onclick: move |_| { word_signal.write().text = "".to_string(); },
                    "✕"
                }
            }
            input {
                class: "word-text-input",
                value: word_signal.read().text.clone(),
                placeholder: "Edit text...",
                onchange: move |ev| { word_signal.write().text = ev.value(); },
            }
            select {
                class: "role-select",
                onchange: move |ev| {
                    let new_role = match ev.value().as_str() {
                        "title" => Role::Title,
                        "ingredient" => Role::Ingredient,
                        "measurement" => Role::Measurement,
                        "unit" => Role::Unit,
                        "instruction" => Role::Instruction,
                        "list_item" => Role::ListItem,
                        "comment" => Role::Comment,
                        _ => Role::Unlabeled,
                    };
                    word_signal.write().role = new_role;
                },
                for role in &roles {
                    option {
                        value: role.label(),
                        selected: word_signal.read().role == *role,
                        "{role.label()}"
                    }
                }
            }
        }
    }
}

#[component]
fn legend() -> Element {
    let roles = [
        Role::Title, Role::Ingredient, Role::Measurement,
        Role::Unit, Role::Instruction, Role::ListItem,
        Role::Comment, Role::Unlabeled,
    ];
    rsx! {
        div { class: "legend",
            for role in &roles {
                div { class: "legend-item",
                    div {
                        class: "legend-swatch",
                        style: format!("background-color: {}", role.color()),
                    }
                    span { class: "legend-label", "{role.label()}" }
                }
            }
        }
    }
}

#[component]
fn root() -> Element {
    let mut app = use_signal(|| load_data());
    let mut selected = use_signal(|| 0usize);
    let mut draw_mode = use_signal(|| DrawMode::None);
    let mut draw_start = use_signal(|| Option::<(f64, f64)>::None);
    let mut draw_current = use_signal(|| Option::<(f64, f64)>::None);

    let current = selected();
    let total = app.read().annotations.len();

    if total == 0 {
        return rsx! {
            div { class: "empty-state",
                h1 { "No annotations found" }
                p { "Upload a PNG to start labeling." }
            }
        };
    }

    let page_data = &app.read().annotations[current];
    let img_path = &app.read().image_paths[current];
    let has_image = !img_path.is_empty() && Path::new(img_path).exists();
    let img_src = if has_image { format!("file://{}", img_path) } else { String::new() };
    let vb = format!("0 0 {} {}", page_data.width, page_data.height);

    // Mouse handlers for drawing
    let on_mouse_down = move |evt: MouseEvent| {
        let coords = evt.data().element_coordinates();
        draw_mode.set(DrawMode::Drawing);
        draw_start.set(Some((coords.x, coords.y)));
        draw_current.set(Some((coords.x, coords.y)));
    };

    let on_mouse_move = move |evt: MouseEvent| {
        if draw_mode() != DrawMode::Drawing {
            return;
        }
        let coords = evt.data().element_coordinates();
        draw_current.set(Some((coords.x, coords.y)));
    };

    let on_mouse_up = move |evt: MouseEvent| {
        if draw_mode() != DrawMode::Drawing {
            return;
        }

        if let Some((sx, sy)) = draw_start() {
            let coords = evt.data().element_coordinates();
            let ex = coords.x;
            let ey = coords.y;

            let x0 = sx.min(ex) as u32;
            let y0 = sy.min(ey) as u32;
            let x1 = sx.max(ex) as u32;
            let y1 = sy.max(ey) as u32;

            if (x1 - x0) > 5 && (y1 - y0) > 5 {
                let new_word = WordAnnotation {
                    bbox: [x0, y0, x1, y1],
                    text: "?".to_string(),
                    confidence: 0.0,
                    role: Role::Unlabeled,
                    list_pos: None,
                };
                let mut app_mut = app.write();
                app_mut.annotations[current].words.push(new_word);
            }
        }

        draw_mode.set(DrawMode::None);
        draw_start.set(None);
        draw_current.set(None);
    };

    let on_upload = move |_| {
        eprintln!("Upload button clicked!");
        let mut app = app.clone();
        let mut selected = selected.clone();
        if let Some(file) = FileDialog::new().pick_file() {
            eprintln!("File selected: {:?}", file);
            let stem = file.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or("page".to_string());
            let img_path = file.to_string_lossy().to_string();
            let (width, height) = if let Ok(img) = image::open(&file) {
                img.dimensions()
            } else {
                (800, 900)
            };
            let annot = PageAnnotation {
                page: stem,
                image_path: img_path.clone(),
                width,
                height,
                words: Vec::new(),
            };
            let mut app_mut = app.write();
            app_mut.image_paths.push(img_path);
            app_mut.annotations.push(annot);
            selected.set(app_mut.annotations.len() - 1);
            eprintln!("Added page, now {} pages", app_mut.annotations.len());
        } else {
            eprintln!("No file selected");
        }
    };
    // Save handler
    let on_save = move |_| {
        let page = selected();
        let json = serde_json::to_string_pretty(&app.read().annotations[page]).unwrap_or_default();
        let annot_path = format!("{}.annot.json", app.read().annotations[page].page);
        eprintln!("Saving to {}...", annot_path);
        if std::fs::write(&annot_path, &json).is_ok() {
            eprintln!("Saved!");
        } else {
            eprintln!("Write failed.");
        }
    };

    rsx! {
        style { r#"
            * {{ box-sizing: border-box; margin: 0; padding: 0; }}
            body {{ background: #111827; color: #f3f4f6; font-family: system-ui, sans-serif; }}

            .nav {{ background: #1f2937; padding: 12px 16px; display: flex; align-items: center; justify-content: space-between; box-shadow: 0 2px 8px rgba(0,0,0,0.3); }}
            .nav h1 {{ font-size: 18px; font-weight: 700; }}
            .nav-stats {{ color: #9ca3af; font-size: 13px; }}

            .pages {{ padding: 16px; background: #1f2937; border-bottom: 1px solid #374151; }}
            .pages h2 {{ font-size: 14px; font-weight: 600; margin-bottom: 8px; color: #9ca3af; }}
            .page-buttons {{ display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }}
            .page-btn {{ padding: 6px 12px; border-radius: 6px; border: none; cursor: pointer; font-size: 13px; color: white; }}
            .page-btn.active {{ background: #2563eb; }}
            .page-btn.inactive {{ background: #374151; }}
            .page-btn.inactive:hover {{ background: #4b5563; }}

            .legend {{ padding: 12px 16px; display: flex; flex-wrap: wrap; gap: 12px; }}
            .legend-item {{ display: flex; align-items: center; gap: 4px; font-size: 12px; }}
            .legend-swatch {{ width: 12px; height: 12px; border-radius: 3px; }}
            .legend-label {{ color: #d1d5db; }}

            .content {{ display: grid; grid-template-columns: 2fr 1fr; height: calc(100vh - 180px); }}
            .image-panel {{ background: #1f2937; padding: 16px; overflow: auto; }}
            .image-panel h2 {{ font-size: 14px; font-weight: 600; margin-bottom: 12px; color: #9ca3af; }}
            .image-container {{ position: relative; display: inline-block; }}
            .page-image {{ border-radius: 8px; max-width: 100%; }}
            .bbox-overlay {{ position: absolute; top: 0; left: 0; width: 100%; height: 100%; cursor: crosshair; }}
            .bbox-overlay.drawing {{ cursor: crosshair; }}
            .upload-btn {{ padding: 8px 16px; background: #2563eb; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px; font-weight: 600; margin: 4px; }}
            .upload-btn:hover {{ background: #1d4ed8; }}
            .page-btn {{ padding: 6px 12px; border-radius: 6px; border: none; cursor: pointer; font-size: 13px; color: white; margin: 2px; }}
            .page-btn.active {{ background: #2563eb; }}
            .page-btn.inactive {{ background: #374151; }}
            .word-header {{ display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }}
            .word-text {{ font-family: monospace; font-size: 13px; color: white; }}
            .word-conf {{ font-size: 11px; color: #9ca3af; }}
            .role-indicator {{ height: 16px; }}
            .role-select {{ width: 100%; font-size: 13px; background: #1f2937; color: #d1d5db; border: 1px solid #4b5563; border-radius: 4px; padding: 4px 8px; margin-bottom: 4px; }}
            .word-text-input {{ width: 100%; font-size: 13px; background: #1f2937; color: #d1d5db; border: 1px solid #4b5563; border-radius: 4px; padding: 4px 8px; margin-bottom: 4px; }}
            .delete-btn {{ background: #ef4444; color: white; border: none; border-radius: 4px; padding: 2px 8px; cursor: pointer; font-size: 12px; }}

            .save-bar {{ padding: 12px 16px; background: #1f2937; display: flex; justify-content: space-between; align-items: center; }}
            .save-btn {{ padding: 10px 20px; background: #16a34a; color: white; border: none; border-radius: 8px; font-size: 14px; font-weight: 600; cursor: pointer; }}
            .save-btn:hover {{ background: #15803d; }}
            .upload-btn {{ padding: 10px 20px; background: #2563eb; color: white; border: none; border-radius: 8px; font-size: 14px; font-weight: 600; cursor: pointer; }}
            .upload-btn:hover {{ background: #1d4ed8; }}

            .no-image {{ background: #374151; border-radius: 8px; height: 400px; display: flex; flex-direction: column; align-items: center; justify-content: center; color: #9ca3af; gap: 12px; }}
            .hint {{ position: absolute; top: 10px; left: 10px; background: rgba(0,0,0,0.7); color: white; padding: 8px 12px; border-radius: 6px; font-size: 13px; pointer-events: none; }}
        "# }
        div { class: "app-container",
            div { class: "nav",
                h1 { "gliphoread — recipe labeler" }
                span { class: "nav-stats", "Pages: {total} · Words: {page_data.words.len()}" }
            }
            div { class: "pages",
                h2 { "Pages" }
                div { class: "page-buttons",
                    button {
                        style: "background: blue; color: white; padding: 10px; margin: 5px;",
                        "BUTTON TEST NO HANDLER"
                    }
                    for i in 0..total {
                        button {
                            onclick: move |_| selected.set(i),
                            class: if i == current { "page-btn active" } else { "page-btn inactive" },
                            "{app.read().annotations[i].page}"
                        }
                    }
                }
            }
            legend {}
            div { class: "content",
                div { class: "image-panel",
                    h2 { "Page View" }
                    if has_image {
                        div { class: "image-container",
                            img {
                                class: "page-image",
                                src: img_src,
                            }
                            svg {
                                class: if draw_mode() == DrawMode::Drawing { "bbox-overlay drawing" } else { "bbox-overlay" },
                                view_box: vb,
                                onmousedown: on_mouse_down,
                                onmousemove: on_mouse_move,
                                onmouseup: on_mouse_up,
                                for word in &page_data.words {
                                    rect {
                                        key: "{word.bbox[0]}-{word.bbox[1]}",
                                        x: word.bbox[0],
                                        y: word.bbox[1],
                                        width: (word.bbox[2] - word.bbox[0]).max(1),
                                        height: (word.bbox[3] - word.bbox[1]).max(1),
                                        fill: "rgba(255,255,255,0.05)",
                                        stroke: word.role.color(),
                                        "stroke-width": "2",
                                        "stroke-opacity": "0.8",
                                        rx: "2",
                                    }
                                }
                                if let Some((sx, sy)) = draw_start() {
                                    if let Some((ex, ey)) = draw_current() {
                                        rect {
                                            x: sx.min(ex),
                                            y: sy.min(ey),
                                            width: (ex - sx).abs(),
                                            height: (ey - sy).abs(),
                                            fill: "rgba(59, 130, 246, 0.15)",
                                            stroke: "#3b82f6",
                                            "stroke-width": "2",
                                            "stroke-dasharray": "5,5",
                                        }
                                    }
                                }
                            }
                            div { class: "hint", "Click and drag to draw boxes" }
                        }
                    } else {
                        div { class: "no-image",
                            "No image loaded"
                            button {
                                class: "upload-btn",
                                onclick: on_upload.clone(),
                                "Upload PNG"
                            }
                        }
                    }
                }
                div { class: "words-panel",
                    h2 { "Words" }
                    for (idx, word) in page_data.words.iter().enumerate() {
                        word_edit {
                            key: "{idx}",
                            word_signal: Signal::new(word.clone()),
                            idx: idx,
                        }
                    }
                    if page_data.words.is_empty() {
                        p { class: "empty-words", "No words yet. Draw boxes on the image." }
                    }
                }
            }
            div { class: "save-bar",
                button {
                    class: "upload-btn",
                    onclick: on_upload.clone(),
                    "+ Upload"
                }
                button {
                    class: "save-btn",
                    onclick: on_save,
                    "Save"
                }
            }
        }
    }
}

fn main() {
    dioxus::launch(root);
}