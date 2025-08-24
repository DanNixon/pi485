height = 8;
tray_depth = 4;
pcb_thickness = 1.6;

screw_positions = [
  [46.5, 27],
  [46.5, -27],
  [-46.5, -27],
];

difference() {
  union() {
    difference() {
      translate([0, 0, -height]) {
        linear_extrude(height) {
          square([105, 66], center = true);
        }
      }

      translate([0, 0, 0.01 - tray_depth]) {
        linear_extrude(tray_depth) {
          square([101, 62], center = true);
        }
      }
    }

    translate([0, 0, -tray_depth]) {
      for(p = screw_positions) {
        translate(p) {
          cylinder(d = 6, h = abs(tray_depth) - pcb_thickness);
        }
      }
    }
  }

  translate([0, 0, -height - 1]) {
    for(p = screw_positions) {
      translate(p) {
        cylinder(d = 3, h = height + 2);
      }
    }
  }
}
