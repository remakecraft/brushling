//! Feature colors.edit_colors: Colors > Edit Colors... and the swatch
//! double-click open a chooser that redefines one swatch (the one most
//! recently clicked, swatch 1 initially). The collapsed dialog offers the 48
//! basic colours and 16 custom colours; "Define Custom Colors >>" enables the
//! Red/Green/Blue and Hue/Sat/Lum fields (HSL scaled to 0-240) and the
//! "Add to Custom Colors" button. Swatches and custom colours live in `State`.

use crate::dialogs::{store_field, DialogAction, OpenDialog, Outcome};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

const EXPAND: &str = "Define Custom Colors >>";
const ADD: &str = "Add to Custom Colors";
const BASIC: &str = "Basic color";
const RGB_FIELDS: [&str; 3] = ["Red", "Green", "Blue"];
const HSL_FIELDS: [&str; 3] = ["Hue", "Sat", "Lum"];

/// The 48 basic colours (6 rows x 8): the 28 default swatch colours first,
/// then 20 further ones.
pub const BASIC_COLORS: [Rgb; 48] = {
    let mut all = [Rgb::new(0, 0, 0); 48];
    let mut i = 0;
    while i < 28 {
        all[i] = super::color_box::DEFAULT_SWATCHES[i];
        i += 1;
    }
    let extra = [
        Rgb::new(255, 128, 128),
        Rgb::new(255, 192, 128),
        Rgb::new(255, 255, 192),
        Rgb::new(192, 255, 192),
        Rgb::new(192, 255, 255),
        Rgb::new(192, 192, 255),
        Rgb::new(255, 192, 255),
        Rgb::new(64, 64, 64),
        Rgb::new(160, 160, 160),
        Rgb::new(224, 224, 224),
        Rgb::new(128, 64, 64),
        Rgb::new(128, 96, 64),
        Rgb::new(64, 128, 64),
        Rgb::new(64, 128, 128),
        Rgb::new(64, 64, 128),
        Rgb::new(128, 64, 128),
        Rgb::new(255, 128, 0),
        Rgb::new(128, 255, 0),
        Rgb::new(0, 128, 64),
        Rgb::new(128, 0, 255),
    ];
    let mut j = 0;
    while j < 20 {
        all[28 + j] = extra[j];
        j += 1;
    }
    all
};

pub fn register(r: &mut Registry) {
    r.command("Colors > Edit Colors...", open_command);
}

fn open_command(st: &mut State) -> HResult<()> {
    open(st);
    Ok(())
}

/// Open the dialog for the swatch most recently clicked.
pub fn open(st: &mut State) {
    let colour = st.swatches[st.last_swatch];
    let mut fields: Vec<DialogField> = RGB_FIELDS
        .iter()
        .chain(&HSL_FIELDS)
        .map(|n| OpenDialog::field(n, "0"))
        .collect();
    for (i, c) in st.custom_colors.iter().enumerate() {
        fields.push(OpenDialog::field(&format!("Custom {}", i + 1), &format!("{},{},{}", c.r, c.g, c.b)));
    }
    fields.push(OpenDialog::field(BASIC, ""));
    let dialog = Dialog {
        kind: DialogKind::EditColors,
        title: "Edit Colors".to_owned(),
        text: String::new(),
        buttons: vec![EXPAND.to_owned(), "OK".to_owned(), "Cancel".to_owned()],
        fields,
    };
    let mut od = OpenDialog::new(dialog, handle);
    set_colour(&mut od, colour);
    // The numeric fields only work once the custom-colour panel is open.
    for f in od.dialog.fields.iter_mut().filter(|f| is_numeric(&f.name)) {
        f.enabled = false;
    }
    st.open_dialog(od);
}

fn is_numeric(name: &str) -> bool {
    RGB_FIELDS.contains(&name) || HSL_FIELDS.contains(&name)
}

fn expanded(od: &OpenDialog) -> bool {
    od.dialog.buttons.iter().any(|b| b == ADD)
}

fn handle(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, value } => {
            set_field(od, field, value)?;
            Ok(Outcome::Keep)
        }
        DialogAction::Click(EXPAND) => {
            if !expanded(od) {
                for f in od.dialog.fields.iter_mut().filter(|f| is_numeric(&f.name)) {
                    f.enabled = true;
                }
                od.dialog.buttons.insert(1, ADD.to_owned());
            }
            Ok(Outcome::Keep)
        }
        DialogAction::Click(ADD) => {
            let c = current(od);
            let slot = st.custom_next % st.custom_colors.len();
            st.custom_colors[slot] = c;
            st.custom_next = (slot + 1) % st.custom_colors.len();
            if let Some(f) = od.field_mut(&format!("Custom {}", slot + 1)) {
                f.value = format!("{},{},{}", c.r, c.g, c.b);
            }
            Ok(Outcome::Keep)
        }
        DialogAction::Click("OK") => {
            let c = current(od);
            st.swatches[st.last_swatch] = c;
            st.fg = c;
            Ok(Outcome::Close)
        }
        DialogAction::Click(_) => Ok(Outcome::Close),
    }
}

fn number(od: &OpenDialog, name: &str) -> i64 {
    od.dialog.field(name).and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

/// The colour the RGB fields currently show.
fn current(od: &OpenDialog) -> Rgb {
    let c = |n| number(od, n).clamp(0, 255) as u8;
    Rgb::new(c("Red"), c("Green"), c("Blue"))
}

fn put(od: &mut OpenDialog, name: &str, value: impl ToString) {
    if let Some(f) = od.field_mut(name) {
        f.value = value.to_string();
    }
}

/// Show `c` in the RGB and HSL fields.
fn set_colour(od: &mut OpenDialog, c: Rgb) {
    put(od, "Red", c.r);
    put(od, "Green", c.g);
    put(od, "Blue", c.b);
    let (h, s, l) = rgb_to_hsl(c);
    put(od, "Hue", h);
    put(od, "Sat", s);
    put(od, "Lum", l);
}

fn parse_number(field: &str, value: &str) -> HResult<i64> {
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| HarnessError::Invalid(format!("{value:?} is not a number for {field:?}")))
}

fn set_field(od: &mut OpenDialog, field: &str, value: &str) -> HResult<()> {
    if RGB_FIELDS.contains(&field) {
        let v = parse_number(field, value)?.clamp(0, 255);
        store_field(od, field, &v.to_string())?;
        let c = current(od);
        set_colour(od, c);
    } else if HSL_FIELDS.contains(&field) {
        let n = parse_number(field, value)?;
        let v = if field == "Hue" { n.rem_euclid(240) } else { n.clamp(0, 240) };
        store_field(od, field, &v.to_string())?;
        let c = hsl_to_rgb(number(od, "Hue"), number(od, "Sat"), number(od, "Lum"));
        put(od, "Red", c.r);
        put(od, "Green", c.g);
        put(od, "Blue", c.b);
    } else if field == BASIC {
        let n = parse_number(field, value)?;
        let Some(c) = usize::try_from(n - 1).ok().and_then(|i| BASIC_COLORS.get(i)) else {
            return Err(HarnessError::Invalid(format!("no basic colour {n}")));
        };
        store_field(od, field, &n.to_string())?;
        set_colour(od, *c);
    } else if let Some(c) = field.strip_prefix("Custom ").and_then(|_| parse_triplet(value)) {
        store_field(od, field, &format!("{},{},{}", c.r, c.g, c.b))?;
        set_colour(od, c);
    } else {
        return Err(HarnessError::Invalid(format!("dialog has no usable field {field:?} for {value:?}")));
    }
    Ok(())
}

fn parse_triplet(value: &str) -> Option<Rgb> {
    let mut parts = value.split(',').map(|p| p.trim().parse::<i64>().ok());
    let (r, g, b) = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then(|| Rgb::new(r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8))
}

/// Standard RGB to HSL, scaled so hue, saturation and luminosity span 0-240
/// (hue wraps at 240).
pub fn rgb_to_hsl(c: Rgb) -> (i64, i64, i64) {
    let (r, g, b) = (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h * 60.0, s)
    };
    ((h / 360.0 * 240.0).round() as i64 % 240, (s * 240.0).round() as i64, (l * 240.0).round() as i64)
}

/// Inverse of [`rgb_to_hsl`] for 0-240 scaled inputs.
pub fn hsl_to_rgb(h: i64, s: i64, l: i64) -> Rgb {
    let h = h.rem_euclid(240) as f64 / 240.0 * 360.0;
    let s = s.clamp(0, 240) as f64 / 240.0;
    let l = l.clamp(0, 240) as f64 / 240.0;
    let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = chroma * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = l - chroma / 2.0;
    let q = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(q(r), q(g), q(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_convert() {
        assert_eq!(rgb_to_hsl(Rgb::new(255, 0, 0)), (0, 240, 120));
        assert_eq!(rgb_to_hsl(Rgb::new(0, 255, 0)), (80, 240, 120));
        assert_eq!(rgb_to_hsl(Rgb::new(0, 0, 255)), (160, 240, 120));
        assert_eq!(rgb_to_hsl(Rgb::new(255, 255, 255)), (0, 0, 240));
        assert_eq!(hsl_to_rgb(80, 240, 120), Rgb::new(0, 255, 0));
        assert_eq!(hsl_to_rgb(0, 0, 0), Rgb::new(0, 0, 0));
    }

    #[test]
    fn basic_grid_starts_with_the_defaults() {
        assert_eq!(BASIC_COLORS[10], Rgb::new(0, 128, 255));
        assert_eq!(BASIC_COLORS[27], Rgb::new(255, 128, 64));
    }
}
