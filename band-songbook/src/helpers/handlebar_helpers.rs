use crate::chords::bar_numbering::barcount_map_of_structure;
use crate::chords::glyph::glyph_of_baritem;
use crate::chords::model::{Bar, BarItem};
use crate::chords::parse::parse;
use crate::model::StructureItem;
use handlebars::{
    Context, Handlebars, Helper, Output, RenderContext, RenderError, RenderErrorReason,
};

/// Returns the length of an array
pub fn len_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let param = h.param(0).and_then(|v| v.value().as_array());
    let len = param.map(|arr| arr.len()).unwrap_or(0);
    out.write(&len.to_string())?;
    Ok(())
}

/// Returns the number of bars in a chord string
pub fn number_of_bars_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let count = parse(input)
        .map_err(|e| RenderErrorReason::Other(e.to_string()))?
        .bars
        .len();
    out.write(&count.to_string())?;
    Ok(())
}

/// Returns the content of a bar at a given index
pub fn bar_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let index = h
        .param(1)
        .and_then(|v| v.value().as_u64())
        .ok_or(RenderErrorReason::Other(
            "missing index parameter".to_string(),
        ))? as usize;

    let result = parse(input)
        .ok()
        .and_then(|parsed| parsed.bars.get(index).cloned())
        .map(|bar| {
            bar.items
                .iter()
                .map(|item| match item {
                    BarItem::Chord(chord) => chord.name.clone(),
                    BarItem::Rest(_) => "HRest".to_string(),
                    BarItem::HalfBar => "2/4".to_string(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    out.write(&result)?;
    Ok(())
}

/// Returns the glyph of a bar at a given index
pub fn bar_glyph_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let index = h
        .param(1)
        .and_then(|v| v.value().as_u64())
        .ok_or(RenderErrorReason::Other(
            "missing index parameter".to_string(),
        ))? as usize;

    let result = parse(input)
        .ok()
        .and_then(|parsed| parsed.bars.get(index).cloned())
        .map(|bar| {
            bar.items
                .iter()
                .map(glyph_of_baritem)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    out.write(&result)?;
    Ok(())
}

/// Draws rectangles for each bar in a row with glyphs
/// Usage: {{bar_rects row color}}
pub fn bar_rects_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let color = h
        .param(1)
        .and_then(|v| v.value().as_str())
        .unwrap_or("gray");

    let parsed = parse(input).map_err(|e| RenderErrorReason::Other(e.to_string()))?;

    for (i, bar) in parsed.bars.iter().enumerate() {
        render_bar(i, bar, color, out)?;
    }

    Ok(())
}

/// Renders a single bar cell as TikZ commands.
fn render_bar(i: usize, bar: &Bar, color: &str, out: &mut dyn Output) -> Result<(), RenderError> {
    let items = &bar.items;

    // Case: [Chord, HalfBar] — chord in upper-left triangle, right triangle grayed out
    if items.len() == 2
        && let (Some(first), Some(BarItem::HalfBar)) = (items.first(), items.get(1))
    {
        let chord_glyph = glyph_of_baritem(first);
        // Draw the full rectangle
        let draw_cmd = format!(
            "    \\draw[draw=black, fill={color}] (\\columnleft + {i}*\\xr, \\currentline) rectangle ++(\\xr, -\\yr);\n"
        );
        out.write(&draw_cmd)?;
        // Fill the lower-right triangle with gray
        let gray_cmd = format!(
            "    \\fill[fill=black!30] (\\columnleft + {i}*\\xr + \\xr, \\currentline) -- ++(0, -\\yr) -- ++(-\\xr, 0) -- cycle;\n"
        );
        out.write(&gray_cmd)?;
        // Draw the diagonal line
        let oblique_cmd = format!(
            "    \\draw[draw=black] (\\columnleft + {i}*\\xr + \\xr, \\currentline) -- ++(-\\xr, -\\yr);\n"
        );
        out.write(&oblique_cmd)?;
        // Place chord in the upper-left triangle
        let chord_cmd = format!(
            "    \\node at (\\columnleft + {}*\\xr + 0.4*\\xr, \\currentline - 0.3*\\yr) {{ {chord_glyph} }};\n",
            i
        );
        out.write(&chord_cmd)?;
        return Ok(());
    }

    if items.len() >= 2 {
        let glyphs: Vec<_> = items.iter().map(glyph_of_baritem).collect();
        // Two or more chords: draw rectangle, then position first chord up 10%, second down 10%
        let draw_cmd = format!(
            "    \\draw[draw=black, fill={color}] (\\columnleft + {i}*\\xr, \\currentline) rectangle ++(\\xr, -\\yr);\n"
        );
        out.write(&draw_cmd)?;

        // First chord shifted towards upper left corner (10% left, 10% up)
        let first_chord_cmd = format!(
            "    \\node at (\\columnleft + {}*\\xr + 0.4*\\xr, \\currentline - 0.3*\\yr) {{ {} }};\n",
            i, glyphs[0]
        );
        out.write(&first_chord_cmd)?;

        // Second chord shifted towards lower right corner (10% right, 10% down)
        let second_chord_cmd = format!(
            "    \\node at (\\columnleft + {}*\\xr + 0.8*\\xr, \\currentline - 0.7*\\yr) {{ {} }};\n",
            i, glyphs[1]
        );
        out.write(&second_chord_cmd)?;

        // Draw oblique bar
        let oblique_cmd = format!(
            "    \\draw[draw=black] (\\columnleft + {i}*\\xr + \\xr, \\currentline) -- ++(-\\xr, -\\yr);\n"
        );
        out.write(&oblique_cmd)?;
    } else {
        // Single chord: original behavior
        let glyphs: Vec<_> = items.iter().map(glyph_of_baritem).collect();
        let glyph = glyphs.join(" ");
        let draw_cmd = format!(
            "    \\draw[draw=black, fill={color}] (\\columnleft + {i}*\\xr, \\currentline) rectangle ++(\\xr, -\\yr) node[midway] {{ {glyph} }};\n"
        );
        out.write(&draw_cmd)?;
    }

    Ok(())
}

/// Returns the repeat multiplier from a chord row (e.g., "Em|G|x2" returns 2)
/// Usage: {{row_multiplier row}}
pub fn row_multiplier_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let multiplier = parse(input).map(|parsed| parsed.repeat.n).unwrap_or(1);
    out.write(&multiplier.to_string())?;
    Ok(())
}

/// Writes a TikZ coordinate `BAR_<n>` for each of `count` consecutive bars.
/// `n` is the running `\barcount` plus the bar's offset, so the names match
/// the bar numbers printed on the chart. `center(k)` is the TikZ point of the
/// k-th bar.
fn write_bar_coordinates(
    count: usize,
    center: impl Fn(usize) -> String,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    for k in 0..count {
        out.write(&format!(
            "    \\pgfmathtruncatemacro{{\\barnum}}{{\\barcount + {k}}}\\coordinate (BAR_\\barnum) at ({});\n",
            center(k)
        ))?;
    }
    Ok(())
}

/// Defines `BAR_<n>` at the center of each bar cell of a row. A repeated row
/// is drawn once, so the bars of every repeat share the cells of the first.
/// Usage: {{{bar_coordinates row}}}
pub fn bar_coordinates_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let input = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing input parameter".to_string(),
        ))?;

    let parsed = parse(input).map_err(|e| RenderErrorReason::Other(e.to_string()))?;
    let nbars = parsed.bars.len();
    write_bar_coordinates(
        nbars * parsed.repeat.n as usize,
        |k| {
            format!(
                "\\columnleft + {}*\\xr + 0.5*\\xr, \\currentline - 0.5*\\yr",
                k % nbars
            )
        },
        out,
    )
}

/// Defines `BAR_<n>` for every bar of a Ref section, all at the center of
/// its box.
/// Usage: {{{ref_bar_coordinates link song.structure}}}
pub fn ref_bar_coordinates_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let count = ref_bar_count_of(h)?;
    write_bar_coordinates(
        count.max(0) as usize,
        |_| "\\columnleft + 2*\\xr, \\currentline - 0.25*\\yr".to_string(),
        out,
    )
}

/// Returns the total bar count for a referenced section (by link/id)
/// Calculates sum of (number_of_bars * repeat) for each row in the referenced Chords section
/// Usage: {{ref_bar_count link song.structure}}
pub fn ref_bar_count_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    out.write(&ref_bar_count_of(h)?.to_string())?;
    Ok(())
}

/// Bar count of the section linked by a `link song.structure` helper call.
fn ref_bar_count_of(h: &Helper) -> Result<i32, RenderError> {
    let link = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderErrorReason::Other(
            "missing link parameter".to_string(),
        ))?;

    let structure_json = h.param(1).ok_or(RenderErrorReason::Other(
        "missing structure parameter".to_string(),
    ))?;

    // Deserialize structure into Vec<StructureItem>
    let structure: Vec<StructureItem> = serde_json::from_value(structure_json.value().clone())
        .map_err(|e| RenderErrorReason::Other(format!("failed to deserialize structure: {e}")))?;

    let bar_numbers = barcount_map_of_structure(&structure);

    // Get the bar count for the linked section
    let total_bars = bar_numbers
        .get(link)
        .map(|(row_numbers, end_count)| {
            row_numbers
                .first()
                .map(|start| end_count - start)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    Ok(total_bars)
}

/// Registers all custom helpers with the handlebars instance
pub fn register_helpers(handlebars: &mut Handlebars) {
    handlebars.register_helper("len-helper", Box::new(len_helper));
    handlebars.register_helper("number_of_bars", Box::new(number_of_bars_helper));
    handlebars.register_helper("bar", Box::new(bar_helper));
    handlebars.register_helper("bar_glyph", Box::new(bar_glyph_helper));
    handlebars.register_helper("bar_rects", Box::new(bar_rects_helper));
    handlebars.register_helper("row_multiplier", Box::new(row_multiplier_helper));
    handlebars.register_helper("ref_bar_count", Box::new(ref_bar_count_helper));
    handlebars.register_helper("bar_coordinates", Box::new(bar_coordinates_helper));
    handlebars.register_helper("ref_bar_coordinates", Box::new(ref_bar_coordinates_helper));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_of_bars_helper() {
        let mut handlebars = Handlebars::new();
        register_helpers(&mut handlebars);

        let template = "{{number_of_bars input}}";
        // "A|B|" has 2 bars (empty bars are filtered out)
        let data = serde_json::json!({"input": "A|B|"});

        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_bar_coordinates_helper() {
        let mut handlebars = Handlebars::new();
        register_helpers(&mut handlebars);

        // 2 bars repeated twice: 4 coordinates, the repeat reusing the cells
        let template = "{{{bar_coordinates input}}}";
        let data = serde_json::json!({"input": "A|B|x2"});

        let result = handlebars.render_template(template, &data).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 4);
        for (k, cell) in [0, 1, 0, 1].into_iter().enumerate() {
            assert!(lines[k].contains(&format!("{{\\barcount + {k}}}\\coordinate (BAR_\\barnum)")));
            assert!(lines[k].contains(&format!("(\\columnleft + {cell}*\\xr + 0.5*\\xr,")));
        }
    }
}
