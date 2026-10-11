"""Erzeugt eine Tanita-Karte mit frei erfundenen Messwerten für Screenshots."""
import math, os, random, sys
from datetime import date, datetime, timedelta

def line(body):
    return body + "CS,%02X" % (sum(body.encode()) & 255)

def profile(slot_dir, slot, birth, gender, height):
    body = '{0,16,~1,2,~3,4,MO,"BC-601",DB,"%s",Bt,0,GE,%d,Hm,%.1f,AL,2,' % (birth.strftime("%d/%m/%Y"), gender, height)
    open(os.path.join(slot_dir, "SYSTEM", "PROF%d.CSV" % slot), "w", newline="").write(line(body) + "\r\n")

def measurements(root, slot, seed, birth, gender, height, start, end, w0, w1, f0, f1, every):
    rnd = random.Random(seed)
    rows, d = [], start
    total = (end - start).days
    while d <= end:
        t = (d - start).days / total
        ease = t * t * (3 - 2 * t)
        w = w0 + (w1 - w0) * ease + 0.9 * math.sin(t * 19) + rnd.gauss(0, 0.35)
        fat = f0 + (f1 - f0) * ease + 0.6 * math.sin(t * 23 + 1) + rnd.gauss(0, 0.5)
        lean = w * (1 - fat / 100)
        bone = round(lean * 0.05, 1)
        muscle = lean - bone
        water = (100 - fat) * 0.725 + rnd.gauss(0, 0.2)
        age = (d - birth).days // 365
        met = age + (fat - (20 if gender == 1 else 30)) * 1.3 + rnd.gauss(0, 0.8)
        visc = max(1, round((fat - (8 if gender == 1 else 20)) / 2.0))
        dci = 370 + 21.6 * lean + 1150
        seg = lambda base, s: base + rnd.gauss(0, s)
        ts = datetime(d.year, d.month, d.day, 7, 0) + timedelta(minutes=rnd.randint(0, 150), seconds=rnd.randint(0, 59))
        body = ('{0,16,~0,2,~1,2,~2,3,~3,4,MO,"BC-601",DT,"%s",Ti,"%s",Bt,0,GE,%d,AG,%d,Hm,%.1f,AL,2,'
                'Wk,%.1f,MI,%.1f,FW,%.1f,Fr,%.1f,Fl,%.1f,FR,%.1f,FL,%.1f,FT,%.1f,'
                'mW,%.1f,mr,%.1f,ml,%.1f,mR,%.1f,mL,%.1f,mT,%.1f,bW,%.1f,IF,%d,rD,%d,rA,%d,ww,%.1f,') % (
            ts.strftime("%d/%m/%Y"), ts.strftime("%H:%M:%S"), gender, age, height,
            w, w / (height / 100) ** 2, fat, seg(fat - 3, .4), seg(fat - 2.6, .4), seg(fat - 1.5, .4), seg(fat - 1.2, .4), seg(fat + 1.8, .4),
            muscle, muscle * 0.058, muscle * 0.057, muscle * 0.172, muscle * 0.170, muscle * 0.543, bone, visc, round(dci), round(max(18, met)), water)
        rows.append(line(body))
        d += timedelta(days=rnd.choice(every))
    open(os.path.join(root, "DATA", "DATA%d.CSV" % slot), "w", newline="").write("\r\n".join(rows) + "\r\n")
    return len(rows)

root = sys.argv[1]
for sub in ("DATA", "SYSTEM"):
    os.makedirs(os.path.join(root, sub), exist_ok=True)
end = date(2026, 10, 10)
a = date(1985, 4, 12); b = date(1990, 9, 3)
profile(root, 1, a, 1, 182.0); profile(root, 2, b, 2, 168.0)
print(measurements(root, 1, 7, a, 1, 182.0, date(2025, 3, 1), end, 93.5, 84.2, 27.5, 21.8, [1, 1, 2, 2, 3, 4]),
      measurements(root, 2, 11, b, 2, 168.0, date(2025, 9, 1), end, 66.0, 63.5, 29.5, 27.0, [2, 3, 3, 5, 7]))
