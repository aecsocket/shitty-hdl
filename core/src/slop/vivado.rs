pub(super) fn quote(value: &str) -> Result<String, crate::ir::Error> {
    let mut word = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => word.push_str("\\\\"),
            '"' => word.push_str("\\\""),
            '$' => word.push_str("\\$"),
            '[' => word.push_str("\\["),
            ']' => word.push_str("\\]"),
            '\n' => word.push_str("\\n"),
            '\r' => word.push_str("\\r"),
            '\t' => word.push_str("\\t"),
            ch if ch.is_control() => {
                return Err(crate::ir::Error(format!(
                    "unsupported control character in Tcl word: U+{:04X}",
                    ch as u32
                )));
            }
            ch => word.push(ch),
        }
    }
    word.push('"');
    Ok(word)
}

pub(super) fn build_script(
    top: &str,
    part: &str,
    design_properties: &[(&str, &str)],
) -> Result<String, crate::ir::Error> {
    if top.trim().is_empty() {
        return Err(crate::ir::Error("Vivado top must not be empty".into()));
    }
    if part.trim().is_empty() {
        return Err(crate::ir::Error("Vivado part must not be empty".into()));
    }
    let top = quote(top)?;
    let part = quote(part)?;
    let mut script = format!(
        "read_verilog -sv top.sv\nread_xdc top.xdc\nsynth_design -top {top} -part {part}\nopt_design\nplace_design\nphys_opt_design\nroute_design\n"
    );
    for &(name, value) in design_properties {
        if name.trim().is_empty() {
            return Err(crate::ir::Error(
                "Vivado design property name must not be empty".into(),
            ));
        }
        let name = quote(name)?;
        let value = quote(value)?;
        script.push_str(&format!("set_property {name} {value} [current_design]\n"));
    }
    script.push_str(
        "report_utilization -file top_utilization.rpt\n\
         report_timing_summary -file top_timing_summary.rpt\n\
         report_drc -file top_drc.rpt\n\
         set setup_paths [get_timing_paths -delay_type max -max_paths 1 -nworst 1]\n\
         if {[llength $setup_paths] > 0} {\n\
             if {[get_property SLACK [lindex $setup_paths 0]] < 0} {\n\
                 error {Setup timing violation}\n\
             }\n\
         }\n\
         set hold_paths [get_timing_paths -delay_type min -max_paths 1 -nworst 1]\n\
         if {[llength $hold_paths] > 0} {\n\
             if {[get_property SLACK [lindex $hold_paths 0]] < 0} {\n\
                 error {Hold timing violation}\n\
             }\n\
         }\n\
         if {[llength [get_drc_violations -filter {SEVERITY == Error}]] > 0} {\n\
             error {Error-severity DRC violations}\n\
         }\n\
         write_bitstream -force top.bit\n",
    );
    Ok(script)
}
