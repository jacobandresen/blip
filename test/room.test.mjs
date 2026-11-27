// The things in the arcade room (web/room.js) stand in the gaps between the machines: no part of any reaches into a
// machine, through the wall or under the floor.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const Room = createRequire(import.meta.url)('../web/room.js');

for (const [name, make] of Object.entries(Room.THINGS)) {
  test(`the ${name} stands clear of the machines, in front of the wall and on the floor`, () => {
    for (const part of make()) {
      const [[x0, x1], [y0], [z0]] = Room.extent(part);
      assert.ok(x0 > Room.GAP[0] && x1 < Room.GAP[1], `${part.kind} at x ${x0}..${x1} is in the gap ${Room.GAP}`);
      assert.ok(y0 >= 0, `${part.kind} is not below the floor (${y0})`);
      assert.ok(z0 >= Room.WALL, `${part.kind} is not behind the wall (${z0})`);
    }
  });
}

test('the sword stands in its stand: its point in the base, its grip in the fork', () => {
  const [base, plinth, , , forkL, forkR, sword] = Room.THINGS.sword();
  // In the sword's picture (80 x 330) the grip is 14 wide, from 52 to 104 down from its top.
  const scale = sword.h / 330, gripTop = sword.y + sword.h - 52 * scale, gripFoot = sword.y + sword.h - 104 * scale;
  assert.ok(sword.y > base.y && sword.y < plinth.y + plinth.h, 'the point is down the slot, inside the stand');
  assert.ok(forkL.x + forkL.w / 2 <= sword.x - 7 * scale && forkR.x - forkR.w / 2 >= sword.x + 7 * scale, 'the fork is either side of the grip');
  assert.ok(forkL.y > gripFoot && forkL.y + forkL.h < gripTop, 'at the grip\'s height');
});
