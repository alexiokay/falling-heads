import * as THREE from './three.module.js';
import { advanceRebound, advanceVertical } from './rebound.mjs';
const $=s=>document.querySelector(s), canvas=$('#game');
const renderer=new THREE.WebGLRenderer({canvas,antialias:true,alpha:false});
renderer.setPixelRatio(Math.min(devicePixelRatio,2)); renderer.shadowMap.enabled=true;renderer.shadowMap.type=THREE.PCFSoftShadowMap;
renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.toneMappingExposure=1.15;
const scene=new THREE.Scene();scene.background=new THREE.Color('#8ba49b');scene.fog=new THREE.FogExp2('#8ba49b',.016);
const camera=new THREE.PerspectiveCamera(57,2,.1,240);
scene.add(new THREE.HemisphereLight(0xd9ebff,0x405126,2.5));
const sun=new THREE.DirectionalLight(0xffe4ae,3.1);sun.position.set(-12,24,12);sun.castShadow=true;sun.shadow.mapSize.set(2048,2048);sun.shadow.camera.left=-24;sun.shadow.camera.right=24;sun.shadow.camera.top=24;sun.shadow.camera.bottom=-24;sun.shadow.camera.far=100;sun.shadow.bias=-.001;scene.add(sun);scene.add(sun.target);
const loader=new THREE.TextureLoader(), mats={};
for(const name of ['hero','stone','sand','grass','wood','leaves','slime']){const t=loader.load(`assets/${name}.${name==='hero'?'jpg':'png'}`);t.magFilter=THREE.NearestFilter;t.wrapS=t.wrapT=THREE.RepeatWrapping;t.colorSpace=THREE.SRGBColorSpace;mats[name]=new THREE.MeshStandardMaterial({map:t,roughness:.95,color:name==='grass'?0x779950:name==='leaves'?0x60834a:0xffffff,alphaTest:name==='leaves'?.4:0});}
const colors={gold:new THREE.MeshStandardMaterial({color:0xffd66c,emissive:0xbd6d12,emissiveIntensity:.3,roughness:.35,metalness:.35}),dark:new THREE.MeshStandardMaterial({color:0x395249}),hazard:new THREE.MeshStandardMaterial({color:0xc07554,roughness:.9}),edge:new THREE.MeshStandardMaterial({color:0xa4dbbe,emissive:0x449475,emissiveIntensity:.35}),hair:new THREE.MeshStandardMaterial({color:0x35261e})};
const cubeGeo=new THREE.BoxGeometry(1,1,1), gemGeo=new THREE.OctahedronGeometry(.3);
const world=new THREE.Group();scene.add(world);
const geometryCache=new Map();
function box(x,y,z,w,h,d,mat,parent=world){const key=[w,h,d].join(',');let geo=geometryCache.get(key);if(!geo){geo=new THREE.BoxGeometry(w,h,d);const uv=geo.attributes.uv;const sizes=[[d,h],[d,h],[w,d],[w,d],[w,h],[w,h]];for(let face=0;face<6;face++)for(let j=0;j<4;j++){const i=face*4+j;uv.setXY(i,uv.getX(i)*Math.max(1,sizes[face][0]/1.5),uv.getY(i)*Math.max(1,sizes[face][1]/1.5));}geometryCache.set(key,geo);}const m=new THREE.Mesh(geo,mats[mat]||colors[mat]);m.position.set(x,y,z);m.castShadow=true;m.receiveShadow=true;parent.add(m);return m;}
const head=new THREE.Group();scene.add(head);box(0,0,0,.92,.92,.92,'hair',head);
// The face stays readable from the chase camera, just like the heads in MineHeads.
const face=new THREE.Mesh(new THREE.PlaneGeometry(.925,.925),mats.hero);face.position.z=.467;head.add(face);
const shadow=new THREE.Mesh(new THREE.CircleGeometry(.55,24),new THREE.MeshBasicMaterial({color:0x172c25,transparent:true,opacity:.3,depthWrite:false}));shadow.rotation.x=-Math.PI/2;scene.add(shadow);
const arch=new THREE.Group();world.add(arch);
let segments=[],hazards=[],gems=[],gateTiles=[],debris=[],rebounds=0,impact=0,mode=0,state='ready',time=0,distance=0,transition=0,score=0,hearts=3,lane=0,x=0,y=.5,vy=0,vx=0,charge=0,invuln=0,paused=false,held={},jumpBuffer=0,crossCooldown=0,returnFrom;
const courseLength=460, speed=12, camPos=new THREE.Vector3(),look=new THREE.Vector3();
const configs=[
{name:'Rebound Run',kicker:'01 / REBOUND RUN',desc:'The head travels forward by itself, bouncing across the corridor and springing away from both walls. Nudge its flight or boost a bounce.',detail:'Closest to the main game: continuous automatic movement, real wall contacts, and upward rebounds. No lanes and no steering required. A / D gently changes the angle; SPACE adds height.',move:'or ← → · nudge the flight',jump:'boost a bounce',objective:'Automatic wall rebounds · Collect crystals · Nudge to reach higher rewards',hint:'Watch it bounce automatically · A / D nudges · SPACE boosts',tint:'#8ba49b'},
{name:'Springline',kicker:'02 / SPRINGLINE',desc:'Free steering and momentum. Hold SPACE to compress against the ground, then release for a longer leap.',detail:'A physical, expressive route. Choose your take-off point and jump strength. Short hops are quick; charged leaps cross wider breaks.',move:'or ← → · steer with momentum',jump:'hold to charge · release to leap',objective:'Steer freely · Charge before wide gaps · Land on the next terrace',hint:'Hold SPACE to charge · Release to leap · A / D to steer',tint:'#b7a18b'},
{name:'Ricochet Ruins',kicker:'03 / RICOCHET RUINS',desc:'Automatic forward bouncing through a ruined passage. Walls and breakable block clusters kick the head into new arcs, with gaps between terraces.',detail:'More like bouncing through the main world: walls redirect the head, blocks shatter on impact, and each contact changes the next arc. Forward travel stays continuous; nudges and jump boosts help cross gaps.',move:'or ← → · nudge the ricochet',jump:'boost across gaps',objective:'Automatic rebounds · Smash amber blocks · Cross broken terraces',hint:'Walls and blocks bounce you automatically · A / D nudges · SPACE boosts',tint:'#849ca4'}];
function makeCourse(){
 while(world.children.length)world.remove(world.children[0]);world.add(arch);arch.clear();segments=[];hazards=[];gems=[];gateTiles=[];debris=[];
 // The opening belongs to the same forest wall as the 2D starting view.
 for(let ix=-10;ix<=10;ix++)for(let iy=0;iy<9;iy++){if(Math.abs(ix)<3&&iy<6)continue;box(ix*1.55,iy*1.55-.7,-4,1.5,1.5,1.5,iy===8?'grass':'stone',arch);}
 for(let i=0;i<7;i++){box(-4.7,i*1.4-.2,-3,1,1.35,1.1,'wood',arch);box(4.7,i*1.4-.2,-3,1,1.35,1.1,'wood',arch);}box(0,8.8,-3,10.4,1,1.2,'wood',arch);
 for(let s of [-1,1])for(let j=0;j<7;j++)box(s*4.05,j*1.1+.1,-2.9,.1,.7,.15,'edge',arch);
 for(let ix=0;ix<6;ix++)for(let iy=0;iy<6;iy++){const bx=(ix-2.5)*1.3,by=iy*1.3+.2;const tile=box(bx,by,-3.7,1.27,1.27,.6,'stone',arch);gateTiles.push({tile,bx,by});}
 box(0,-1.1,4,30,1.2,16,'stone');box(0,-.45,4,30,.15,16,'grass');
 const material=mode===1?'sand':'stone';
 function floor(cx,start,len,width=2.9){box(cx,-.65,-start-len/2,width,1.3,len,material);box(cx,.025,-start-len/2,width,.08,len,mode===1?'sand':'grass');segments.push({x:cx,start,end:start+len,width});}
 function obstacle(cx,z,height=1.7){let m=box(cx,height/2,-z,1.7,height,1.2,material);const cap=box(cx,height+.1,-z,1.78,.17,1.3,mode===2?'gold':'hazard');hazards.push({x:cx,z,height,hit:false,m,cap});}
 function gem(cx,z,height=1.15){const m=new THREE.Mesh(gemGeo,colors.gold);m.position.set(cx,height,-z);world.add(m);gems.push({x:cx,z,height,m,taken:false});}
 for(let l=-1;l<=1;l++)floor(l*3,8,25);
 for(let z=33,i=0;z<courseLength;z+=18,i++){
  if(mode===0){floor(0,z,18,9);for(let j=0;j<5;j++)gem(Math.sin((z+j*3)*.12)*3.3,z+j*3,1.5+(j%2)*1.4);}
  if(mode===1){let cx=Math.sin(i*1.9)*1.55;floor(cx,z,12,6.1);if(i%3===1)obstacle(cx+1.45,z+7);for(let j=0;j<3;j++)gem(cx-1,z+4+j*3,1.3);}
  if(mode===2){floor(0,z,i%3===2?14.5:18,9);const side=i%2===0?-1:1;obstacle(side*2.5,z+8,2.8);obstacle(-side*.8,z+14,1.7);box(side*3,.12,-(z+12),2.4,.18,1.3,'slime');for(let j=0;j<4;j++)gem(side*(3-j*.8),z+j*3,1.5+(j%2));}
 }
 for(let l=-1;l<=1;l++)floor(l*3,courseLength-4,24);
 // Broken colonnades, forest silhouettes and visible side walls give real depth cues.
 for(let z=12,i=0;z<courseLength+40;z+=12,i++)for(let s of [-1,1]){
  const h=6+(i%3)*2;box(s*7,h/2-1,-z,1.7,h,1.7,mode===1?'sand':'wood');
  if(mode!==1){box(s*9,h,-z,6,2.6,6,'leaves');box(s*11,h+2,-z-2,5,2,5,'leaves');}
  else{box(s*7,h-1,-z,2.6,.6,2.6,'sand');box(s*10,-1,-z,5,2,8,'sand');}
  if(i%3===0)box(0,mode===1?h:9.5,-z,15,.7,1.5,material);
  if(mode!==1){box(s*5,4.5,-z,1.2,9,12,'stone');for(const py of [1.5,7.3])box(s*4.37,py,-z,.1,1.9,mode===2?2:6,'slime');box(s*2.25,9,-z,4.5,.45,12,'stone');box(s*2.25,8.74,-z,4.2,.08,2,'slime');}
 }
 box(0,-5,-240,80,1,540,'dark');
 for(let l=-1;l<=1;l++)box(l*3,.07,-courseLength,2.8,.1,1,'slime');
 scene.background.set(configs[mode].tint);scene.fog.color.set(configs[mode].tint);
}
function reset(ready=false){makeCourse();time=0;distance=8;transition=0;score=0;hearts=3;lane=0;x=0;y=.5;vy=0;vx=mode===1?0:5.6;charge=0;invuln=0;jumpBuffer=0;crossCooldown=0;rebounds=0;impact=0;held={};paused=false;state=ready?'ready':'enter';$('#pause').textContent='Pause';$('#cover').style.display=ready?'block':'none';$('#hint').style.display=ready?'none':'block';$('#hint').textContent='The background opens. Follow the head inside.';updateHud();}
function choose(n){mode=n;const c=configs[n];document.querySelectorAll('.concept').forEach((el,i)=>{el.classList.toggle('active',i===n);el.setAttribute('aria-pressed',i===n)});$('#cover-kicker').textContent=c.kicker;$('#cover-title').textContent='Step through the world.';$('#cover-desc').textContent=c.desc;$('#detail').textContent=c.detail;$('#move-label').textContent=c.move;$('#jump-label').textContent=c.jump;$('#objective').textContent=c.objective;$('#start').innerHTML='Enter the rift <span>↗</span>';reset(true);}
function grounded(){return segments.some(s=>distance>=s.start&&distance<=s.end&&Math.abs(x-s.x)<s.width/2+.2);}
function lose(){if(invuln>0)return;hearts--;invuln=1.4;if(hearts<=0){state='over';$('#cover-kicker').textContent='RUN ENDED';$('#cover-title').textContent='One more run?';$('#cover-desc').textContent=`${Math.floor(distance-8)} metres · ${score} crystals. ${configs[mode].hint}`;$('#start').textContent='Try again ↗';$('#cover').style.display='block';$('#hint').style.display='none';}else{if(!grounded()){let nearest=segments.filter(s=>s.end>distance&&s.start<distance+20).sort((a,b)=>Math.abs(a.x-x)-Math.abs(b.x-x))[0];if(nearest){x=nearest.x;lane=Math.round(x/3);distance=Math.max(distance,nearest.start+1);}}y=2.5;vy=5;}updateHud();}
function updateHud(){$('#score').textContent=`${score} crystals${mode!==1?' · '+rebounds+' rebounds':''}`;$('#health').textContent='♥ '.repeat(Math.max(0,hearts))+'♡ '.repeat(3-Math.max(0,hearts));$('#progress').style.width=Math.min(100,(distance-8)/(courseLength-8)*100)+'%';}
function input(key,down){if(key==='pause'&&down){togglePause();return}if(state!=='run')return;if(down&&!held[key]){
 if(key==='jump'){jumpBuffer=.22;if(mode!==1&&crossCooldown<=0){vy=Math.max(vy,9.5);crossCooldown=.7;}}
 }if(key==='jump'&&!down&&mode===1){if(y<1.5||charge>0){vy=6+Math.min(1,charge)*7.5;charge=0;}jumpBuffer=0;}held[key]=down;}
function togglePause(){if(!['run','enter','return'].includes(state))return;paused=!paused;$('#pause').textContent=paused?'Resume':'Pause';$('#hint').textContent=paused?'Paused':configs[mode].hint;$('#hint').style.display=paused?'block':'none';}
const ease=t=>t*t*(3-2*t), v=(a,b,t)=>a+(b-a)*t;
function frame(dt){
 if(!paused){time+=dt;invuln=Math.max(0,invuln-dt);crossCooldown=Math.max(0,crossCooldown-dt);impact=Math.max(0,impact-dt*5);
 if(state==='enter'){transition=Math.min(1,transition+dt/3.2);if(transition===1){state='run';$('#hint').textContent=configs[mode].hint;time=0;}}
 if(state==='run'){
  distance+=speed*dt;jumpBuffer=Math.max(0,jumpBuffer-dt);
  if(mode===1){const accel=(held.right?1:0)-(held.left?1:0);vx+=accel*28*dt;vx*=Math.exp(-5*dt);x=THREE.MathUtils.clamp(x+vx*dt,-4.3,4.3);}
  else{const next=advanceRebound(x,vx,dt,(held.right?1:0)-(held.left?1:0),3.87,mode===2?6.6:5.6);x=next.x;vx=next.vx;if(next.wall){rebounds++;impact=1;}}
  const on=grounded();
  if(mode===1&&held.jump&&y<1&&on&&vy<=0){y=.5;vy=0;charge=Math.min(1,charge+dt/0.65);}
  else if(mode===1){vy-=24*dt;y+=vy*dt;if(y<=.5&&on&&vy<0){y=.5;vy=jumpBuffer>0?10:4.1;}}
  else{const next=advanceVertical(y,vy,dt,.5,8.25,on);y=next.y;vy=next.vy;if(next.contact){rebounds++;impact=1;}}
  if(y< -3)lose();
  for(const h of hazards)if(!h.hit&&Math.abs(distance-h.z)<1.05&&Math.abs(x-h.x)<1.3&&y<h.height+.46){h.hit=true;if(mode===1)lose();else{vx=(Math.sign(x-h.x)||-Math.sign(vx)||1)*6.6;vy=8.5;rebounds++;impact=1;h.m.visible=false;h.cap.visible=false;for(let i=0;i<7;i++){const bit=box(h.x+(i%3-1)*.38,.5+Math.floor(i/3)*.5,-h.z,.38,.38,.38,'stone');debris.push({m:bit,life:1.2,v:new THREE.Vector3(Math.sin(i*2)*4,3+i*.5,Math.cos(i)*3)});}}}
  for(const b of debris){b.life-=dt;b.v.y-=15*dt;b.m.position.addScaledVector(b.v,dt);b.m.rotation.x+=dt*3;b.m.rotation.z+=dt*2;b.m.scale.setScalar(Math.max(0,Math.min(1,b.life*2)));if(b.life<=0)world.remove(b.m);}debris=debris.filter(b=>b.life>0);
  for(const g of gems)if(!g.taken&&Math.abs(distance-g.z)<1.05&&Math.abs(x-g.x)<.9&&Math.abs(y-g.height)<1.7){g.taken=true;g.m.visible=false;score++;updateHud();}
  if(distance>=courseLength){state='return';transition=0;returnFrom={x:head.position.x,y:head.position.y,z:head.position.z,cam:camera.position.clone(),look:look.clone()};$('#hint').style.display='block';$('#hint').textContent='Back through the same world.';}
  if(time>5)$('#hint').style.display='none';updateHud();
 }
 if(state==='return'){transition=Math.min(1,transition+dt/2.8);if(transition===1){state='complete';$('#cover-kicker').textContent='RIFT COMPLETE';$('#cover-title').textContent='Back where you began.';$('#cover-desc').textContent=`${score} crystals collected. Try another movement style using the cards above.`;$('#start').textContent='Run it again ↗';$('#cover').style.display='block';$('#hint').style.display='none';}}
 }
 let bob=Math.abs(Math.sin(time*2.6))*.9;
 if(state==='ready'||state==='complete'){head.position.set(-2.9,.8+bob,5);camPos.set(0,5.2,24);look.set(0,3.2,-3);}
 else if(state==='enter'){const t=ease(transition);head.position.set(v(-2.9,0,t),.8+Math.sin(transition*Math.PI)*1.7,v(5,-8,t));camPos.set(0,v(5.2,mode===1?4.4:4.8,t),v(24,1.2,t));look.set(0,v(3.2,mode===1?1.25:4.2,t),v(-3,-21,t));}
 else if(state==='return'){const t=ease(transition);head.position.set(v(returnFrom.x,-2.9,t),v(returnFrom.y,1.5,t)+Math.sin(t*Math.PI)*2,v(returnFrom.z,5,t));camPos.copy(returnFrom.cam).lerp(new THREE.Vector3(0,5.2,24),t);look.copy(returnFrom.look).lerp(new THREE.Vector3(0,3.2,-3),t);}
 else{head.position.set(x,y,-distance);camPos.set(x*.15,mode===1?4.4:4.8,-distance+9.2);look.set(x*.1,mode===1?1.25:4.2,-distance-13);}
 head.rotation.z=-vx*.035;head.rotation.x=0;
 const opening=state==='ready'||state==='complete'?0:state==='enter'?ease(Math.min(1,transition*2.1)):state==='return'?1-ease(Math.max(0,(transition-.55)/.45)):1;
 for(const g of gateTiles){g.tile.position.set(g.bx+Math.sign(g.bx)*opening*(4.8+Math.abs(g.bx)*.25),g.by+opening*.35,-3.7-opening*(2+g.by*.2));g.tile.rotation.y=Math.sign(g.bx)*opening*.35;}
 head.scale.set(1+charge*.23-impact*.12,1-charge*.3+impact*.12,1+charge*.15);head.visible=invuln===0||Math.sin(time*35)>-.5;
 shadow.position.set(head.position.x,.12,head.position.z);shadow.scale.setScalar(Math.max(.5,1-head.position.y*.08));shadow.visible=(state==='run'&&grounded())||state==='ready'||state==='enter';
 camera.position.copy(camPos);camera.lookAt(look);sun.position.set(head.position.x-12,24,head.position.z+12);sun.target.position.set(head.position.x,0,head.position.z-10);
 for(const g of gems){g.m.rotation.y=time*1.6;g.m.rotation.z=.2;}
 $('#phase').textContent=paused?'PAUSED':({ready:'SIDE VIEW',enter:'ENTERING THE WORLD',run:configs[mode].name.toUpperCase(),over:'RUN ENDED',return:'RETURNING',complete:'SIDE VIEW'})[state];
 $('#charge').style.display=mode===1&&state==='run'?'block':'none';$('#charge i').style.width=charge*100+'%';
}
let last=performance.now(),acc=0;function animate(now){requestAnimationFrame(animate);const dt=Math.min((now-last)/1000,.05);last=now;acc+=dt;while(acc>=1/120){frame(1/120);acc-=1/120;}renderer.render(scene,camera);}
new ResizeObserver(()=>{const r=canvas.getBoundingClientRect();renderer.setSize(r.width,r.height,false);camera.aspect=r.width/r.height;camera.updateProjectionMatrix()}).observe(canvas);
document.querySelectorAll('.concept').forEach(b=>b.onclick=()=>choose(+b.dataset.mode));$('#start').onclick=()=>reset();$('#restart').onclick=()=>reset();$('#pause').onclick=togglePause;$('#full').onclick=()=>document.fullscreenElement?document.exitFullscreen():$('.stage').requestFullscreen();
const keys={ArrowLeft:'left',KeyA:'left',ArrowRight:'right',KeyD:'right',Space:'jump',ArrowUp:'jump',KeyW:'jump',KeyP:'pause'};
window.addEventListener('keydown',e=>{if(keys[e.code]&&!['INPUT','TEXTAREA'].includes(e.target.tagName)){e.preventDefault();if(!e.repeat)input(keys[e.code],true);}});window.addEventListener('keyup',e=>{if(keys[e.code]){e.preventDefault();input(keys[e.code],false);}});
window.addEventListener('blur',()=>{held={};if(state==='run'&&!paused)togglePause()});document.addEventListener('visibilitychange',()=>{if(document.hidden&&state==='run'&&!paused)togglePause()});
document.querySelectorAll('[data-key]').forEach(b=>{b.addEventListener('pointerdown',e=>{e.preventDefault();b.setPointerCapture(e.pointerId);input(b.dataset.key,true)});b.addEventListener('pointerup',()=>input(b.dataset.key,false));b.addEventListener('pointercancel',()=>input(b.dataset.key,false));});
// Read-only diagnostics for verifying controls and route states in the preview.
window.runnerSnapshot=()=>({mode:configs[mode].name,state,paused,distance,x,y,vx,rebounds,charge,score,hearts,head:head.position.toArray(),camera:camera.position.toArray()});
choose(1);requestAnimationFrame(animate);
