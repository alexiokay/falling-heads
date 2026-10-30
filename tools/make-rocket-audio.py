"""Original deterministic DSP effects; no samples or third-party recordings."""
import math, random, wave, struct, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
out=root/'assets/audio/rocket';out.mkdir(parents=True,exist_ok=True)
rate=44100
for name,duration in [('ignition',0.75),('engine',2.8),('crash',1.6)]:
 rng=random.Random(724);samples=[];low=0.;phase=0.
 for i in range(int(rate*duration)):
  t=i/rate;u=t/duration;noise=rng.uniform(-1,1);low=low*.88+noise*.12
  if name=='ignition':
   phase+=2*math.pi*(65+170*u*u)/rate
   value=(.35*math.sin(phase)+low*2.4+noise*.08)*min(1,t/.04)*(1-u)**.35
  elif name=='engine':
   phase+=2*math.pi*(83+12*math.sin(t*1.6))/rate
   value=(low*2.+.18*math.sin(phase)+.07*math.sin(phase*2.01))*(.84+.16*math.sin(t*37))*min(1,t/.08)*min(1,(duration-t)/.16)
  else:
   value=(low*4.*math.exp(-t*3)+noise*.7*math.exp(-t*14)+math.sin(2*math.pi*(57*t-9*t*t))*.5*math.exp(-t*6))*min(1,t/.004)
   for delay,freq in [(.08,470),(.17,810),(.26,1240),(.41,630)]:
    if t>delay:value+=.1*math.sin(2*math.pi*freq*(t-delay))*math.exp(-(t-delay)*12)
   value*=min(1,(duration-t)/.1)
  samples.append(value)
 gain=.65/max(abs(x) for x in samples)
 wav=root/'target'/f'rocket-{name}.wav'
 with wave.open(str(wav),'wb') as f:
  f.setparams((1,2,rate,0,'NONE','not compressed'));f.writeframes(b''.join(struct.pack('<h',round(x*gain*32767)) for x in samples))
 subprocess.run(['ffmpeg','-y','-loglevel','error','-i',str(wav),'-c:a','libvorbis','-q:a','5',str(out/f'{name}.ogg')],check=True)
 print(name,len(samples)/rate,'seconds; peak',max(abs(x*gain) for x in samples))
