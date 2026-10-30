// Automatic cross-corridor motion. Steering bends the flight; it never picks a lane.
export function advanceRebound(x, vx, dt, steer, limit, cruise) {
  vx += steer * 11 * dt;
  if (Math.abs(vx) < cruise * .65) vx += (Math.sign(vx) || 1) * 4 * dt;
  vx = Math.max(-cruise * 1.45, Math.min(cruise * 1.45, vx));
  x += vx * dt;
  let wall = 0;
  // Reflect overshoot as well as velocity, preserving travel at the boundary.
  while (x > limit || x < -limit) {
    if (x > limit) { x = 2 * limit - x; vx = -Math.max(cruise, Math.abs(vx)); wall = 1; }
    else { x = -2 * limit - x; vx = Math.max(cruise, Math.abs(vx)); wall = -1; }
  }
  return { x, vx, wall };
}

export function advanceVertical(y, vy, dt, floor, ceiling, grounded) {
  vy -= 3.5 * dt;
  y += vy * dt;
  let contact = 0;
  if (y > ceiling) { y = 2 * ceiling - y; vy = -Math.max(7.5, Math.abs(vy)); contact = 1; }
  if (grounded && y < floor) { y = 2 * floor - y; vy = Math.max(10.5, Math.abs(vy) * .93); contact = -1; }
  return {y, vy, contact};
}
