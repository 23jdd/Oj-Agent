use super::{AppWindow, State, StyleField, Theme, show_frame};
use crate::{
    error::Result,
    style::{Rgb, StyleConfig, invalid},
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

const FIELDS: &[(&str, &str, i32)] = &[
    ("fonts.ui_family", "界面字体", 0),
    ("fonts.code_family", "代码字体", 0),
    ("fonts.animation_family", "动画字体", 0),
    ("fonts.text_size", "正文字号 / 12–18 px", 0),
    ("fonts.label_size", "标签字号 / 12–16 px", 0),
    ("fonts.heading_size", "标题字号 / 18–28 px", 0),
    ("fonts.animation_scale", "动画字号倍率 / 0.8–1.6", 0),
    ("fonts.minimum_text_pixels", "动画最小文字 / 12–18 px", 0),
    ("fonts.line_height", "动画行高 / 1.2–1.8", 0),
    ("colors.background", "背景", 1),
    ("colors.surface", "面板 / 画布", 1),
    ("colors.text_primary", "主要文字", 1),
    ("colors.text_secondary", "次要文字", 1),
    ("colors.border", "边框与阴影", 1),
    ("colors.accent", "强调色 / 动画高亮", 1),
    ("colors.surface_hover", "悬停背景", 1),
    ("colors.surface_active", "按下背景", 1),
    ("colors.focus", "键盘焦点", 1),
    ("colors.text_disabled", "禁用文字", 1),
    ("colors.disabled", "禁用背景", 1),
    ("colors.success", "成功", 1),
    ("colors.danger", "错误", 1),
    ("geometry.border_width", "边框宽度 / 1–3 px", 2),
    ("geometry.shadow_offset", "阴影偏移 / 0–6 px", 2),
    ("geometry.radius", "圆角 / 0–8 px", 2),
    ("geometry.control_height", "控件高度 / 36–48 px", 2),
];
pub fn fields(style: &StyleConfig) -> ModelRc<StyleField> {
    let json = serde_json::to_value(style).expect("validated style serializes");
    ModelRc::new(VecModel::from(
        FIELDS
            .iter()
            .map(|(key, label, group)| {
                let (section, name) = key.split_once('.').unwrap();
                let value = &json[section][name];
                let value = value.as_str().map(str::to_owned).unwrap_or_else(|| {
                    value
                        .as_f64()
                        .map(|number| (number as f32).to_string())
                        .unwrap_or_else(|| value.to_string())
                });
                StyleField {
                    key: (*key).into(),
                    label: (*label).into(),
                    group: *group,
                    is_color: *group == 1,
                    swatch: Rgb::parse(&value).unwrap_or(Rgb([255, 255, 255])).slint(),
                    value: value.into(),
                }
            })
            .collect::<Vec<_>>(),
    ))
}
fn parse(fields: &ModelRc<StyleField>) -> Result<StyleConfig> {
    let mut json = serde_json::to_value(StyleConfig::default()).expect("default style serializes");
    for row in fields.iter() {
        let (section, name) = row
            .key
            .split_once('.')
            .ok_or_else(|| invalid("Unknown style field."))?;
        if !FIELDS.iter().any(|(key, _, _)| *key == row.key.as_str()) {
            return Err(invalid("Unknown style field."));
        }
        let value = row.value.trim();
        if row.is_color {
            Rgb::parse(value)
                .map_err(|_| invalid(format!("{}：请输入 #RRGGBB 颜色。", row.label)))?;
        }
        json[section][name] = if json[section][name].is_number() {
            let number = value
                .parse::<f32>()
                .map_err(|_| invalid(format!("{}：请输入数值。", row.label)))?;
            serde_json::Number::from_f64(f64::from(number))
                .map(serde_json::Value::Number)
                .ok_or_else(|| invalid("Number must be finite."))?
        } else {
            serde_json::Value::String(value.into())
        };
    }
    let style: StyleConfig = serde_json::from_value(json).map_err(|e| invalid(e.to_string()))?;
    style.validate()?;
    Ok(style)
}
pub fn apply(ui: &AppWindow, style: &StyleConfig) {
    let theme = ui.global::<Theme>();
    theme.set_font_family(style.fonts.ui_family.clone().into());
    theme.set_code_font_family(style.fonts.code_family.clone().into());
    theme.set_text_size(style.fonts.text_size);
    theme.set_label_size(style.fonts.label_size);
    theme.set_heading_size(style.fonts.heading_size);
    theme.set_background(style.colors.background.slint());
    theme.set_surface(style.colors.surface.slint());
    theme.set_surface_hover(style.colors.surface_hover.slint());
    theme.set_surface_active(style.colors.surface_active.slint());
    theme.set_border(style.colors.border.slint());
    theme.set_text_primary(style.colors.text_primary.slint());
    theme.set_text_secondary(style.colors.text_secondary.slint());
    theme.set_text_disabled(style.colors.text_disabled.slint());
    theme.set_disabled(style.colors.disabled.slint());
    theme.set_accent(style.colors.accent.slint());
    theme.set_focus(style.colors.focus.slint());
    theme.set_success(style.colors.success.slint());
    theme.set_danger(style.colors.danger.slint());
    theme.set_border_width(style.geometry.border_width);
    theme.set_shadow_offset(style.geometry.shadow_offset);
    theme.set_radius_sm(style.geometry.radius);
    theme.set_control_height(style.geometry.control_height);
}
pub fn bind(ui: &AppWindow, state: Rc<RefCell<State>>, path: PathBuf) {
    ui.set_style_path(path.display().to_string().into());
    let weak = ui.as_weak();
    let current = state.clone();
    ui.on_open_style(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_style_fields(fields(&current.borrow().style));
            ui.set_style_error("".into());
            ui.set_style_open(true);
        }
    });
    let weak = ui.as_weak();
    ui.on_reset_style(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_style_fields(fields(&StyleConfig::default()));
            ui.set_style_error("".into());
        }
    });
    let weak = ui.as_weak();
    ui.on_edit_style(move |index, value| {
        if let Some(ui) = weak.upgrade() {
            let model = ui.get_style_fields();
            if let Some(mut row) = model.row_data(index as usize) {
                if row.is_color {
                    row.swatch = Rgb::parse(&value).map(Rgb::slint).unwrap_or(row.swatch);
                }
                row.value = value;
                model.set_row_data(index as usize, row);
                ui.set_style_error("".into());
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_save_style(move || {
        if let Some(ui) = weak.upgrade() {
            let result = parse(&ui.get_style_fields()).and_then(|style| {
                style.save(&path)?;
                Ok(style)
            });
            match result {
                Ok(style) => {
                    apply(&ui, &style);
                    let mut state = state.borrow_mut();
                    state.style = Arc::new(style);
                    state.preview_epoch = state.preview_epoch.wrapping_add(1);
                    state.last_frame = -1.;
                    ui.set_style_open(false);
                    ui.set_error_text("".into());
                    ui.set_status_text("外观已保存并应用".into());
                    if let Err(error) = show_frame(&ui, &mut state) {
                        super::show_error(&ui, error);
                    }
                }
                Err(error) => ui.set_style_error(error.to_string().into()),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_round_trip_and_invalid_draft_leave_current_config_untouched() {
        let style = StyleConfig::default();
        let model = fields(&style);
        assert_eq!(parse(&model).unwrap(), style);
        let mut row = model.row_data(3).unwrap();
        row.value = "NaN".into();
        model.set_row_data(3, row);
        assert!(parse(&model).is_err());
        assert_eq!(style, StyleConfig::default());
    }
}
