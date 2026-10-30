import assert from 'node:assert/strict';
import {advanceRebound,advanceVertical} from './rebound.mjs';
let x=0,vx=5.6,count=0,left=false,right=false;
for(let i=0;i<120*30;i++){
 const p=advanceRebound(x,vx,1/120,0,3.87,5.6);x=p.x;vx=p.vx;
 assert.ok(Math.abs(x)<=3.87);
 if(p.wall){count++;left ||= p.wall<0;right ||=p.wall>0;assert.equal(Math.sign(vx),-p.wall);}
}
assert.ok(count>15&&left&&right,'No-input motion must keep rebounding from both walls');
const overshoot=advanceRebound(3.8,6,.1,0,3.87,5.6);
assert.ok(overshoot.x<3.87&&overshoot.vx<0,'A crossing must reflect overshoot, not stick outside');
const nudged=advanceRebound(0,5.6,.1,-1,3.87,5.6);
assert.ok(nudged.x>0&&nudged.vx<5.6,'A nudge bends flight without an instant lane change');
console.log('PASS: sustained no-input rebounds, both wall normals, overshoot, gentle steering');
let y=.5,vy=0,floors=0,ceilings=0;
for(let i=0;i<3600;i++){const r=advanceVertical(y,vy,1/120,.5,8.25,true);y=r.y;vy=r.vy;if(r.contact<0)floors++;if(r.contact>0)ceilings++;assert.ok(y>=.5&&y<=8.25);}
assert.ok(floors>5&&ceilings>5,'Vertical motion must reach both floor and ceiling repeatedly');
console.log('PASS: sustained XY flight with floor and ceiling reflections');
