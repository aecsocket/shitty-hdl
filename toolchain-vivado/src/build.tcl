# init args
if {$argc < 5} {
    error "usage: build.tcl OUT_DIR VERILOG XDC TOP_MODULE PART ?VERILOG_LIBRARY_DIR ...?"
}
lassign $argv out_dir verilog xdc top_module part
set out_dir [file normalize $out_dir]
set verilog [file normalize $verilog]
set xdc [file normalize $xdc]
set verilog_dirs {}
foreach dir [lrange $argv 5 end] {
    lappend verilog_dirs [file normalize $dir]
}

# make the design

file mkdir $out_dir
set verilog_sources [list $verilog]
foreach dir $verilog_dirs {
    foreach source [glob -nocomplain -directory $dir *.v] {
        lappend verilog_sources [file normalize $source]
    }
}
set_property include_dirs $verilog_dirs [current_fileset]
set_property verilog_define {BSV_NO_MAIN_V BSV_NO_INITIAL_BLOCKS} [current_fileset]
read_verilog [lsort -unique $verilog_sources]
read_xdc $xdc

puts {shitty_hdl:Synthesis}
flush stdout
synth_design -top $top_module -part $part
write_checkpoint -force [file join $out_dir post_synth.dcp]
report_utilization -file [file join $out_dir utilization.rpt]

puts {shitty_hdl:Optimization}
flush stdout
opt_design

puts {shitty_hdl:Placement}
flush stdout
place_design

puts {shitty_hdl:PhysicalOptimization}
flush stdout
phys_opt_design

puts {shitty_hdl:Routing}
flush stdout
route_design

# generate full reports

write_checkpoint -force [file join $out_dir post_route.dcp]
report_timing_summary -file [file join $out_dir timing.rpt]
report_drc -file [file join $out_dir drc.rpt]

proc worst_slack_json {paths} {
    if {[llength $paths] == 0} {
        return null
    }
    set slack [format %.17g [get_property SLACK [lindex $paths 0]]]
    if {![regexp {^-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$} $slack]} {
        error "cannot encode non-finite timing slack: $slack"
    }
    return $slack
}

set setup_paths [get_timing_paths -quiet -delay_type max -sort_by slack -max_paths 1]
set hold_paths [get_timing_paths -quiet -delay_type min -sort_by slack -max_paths 1]
set drc_error_count [llength [get_drc_violations -quiet -filter {SEVERITY == Error}]]

set timing_json [format \
    {{"worst_setup_slack_ns":%s,"worst_hold_slack_ns":%s,"drc_error_count":%d}} \
    [worst_slack_json $setup_paths] [worst_slack_json $hold_paths] $drc_error_count]
set timing_file [open [file join $out_dir timing.json] w]
fconfigure $timing_file -encoding utf-8
puts $timing_file $timing_json
close $timing_file

puts {shitty_hdl:WriteBitstream}
flush stdout
write_bitstream -force [file join $out_dir top.bit]
