export function toIsoFromEpochSeconds(sec: number | null | undefined): string | null {
  if (!sec || sec <= 0) return null;
  return new Date(sec * 1000).toISOString();
}

// Parses strings like:
// - "3am", "3:15pm"
// - "Feb 28, 10am", "Feb 28, 10:30pm"
// Returns an ISO string (UTC) computed from local time.
export function parseLocalResetAt(body: string, now = new Date()): string | null {
  const year = now.getFullYear();

  const monthMap: Record<string, number> = {
    jan: 0,
    feb: 1,
    mar: 2,
    apr: 3,
    may: 4,
    jun: 5,
    jul: 6,
    aug: 7,
    sep: 8,
    oct: 9,
    nov: 10,
    dec: 11,
  };

  let month: number | null = null;
  let day: number | null = null;
  let timePart = body.trim();

  const md = /^([A-Za-z]{3})\s+(\d{1,2}),\s*(.+)$/.exec(timePart);
  if (md) {
    const mon = md[1];
    const dayStr = md[2];
    const rest = md[3];
    if (mon && dayStr && rest) {
      month = monthMap[mon.toLowerCase()] ?? null;
      day = Number(dayStr);
      timePart = rest;
    }
  }

  const tm = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm)$/i.exec(timePart);
  if (!tm) return null;
  const hhStr = tm[1];
  const mmStr = tm[2];
  const apRaw = tm[3];
  if (!hhStr || !apRaw) return null;

  let hh = Number(hhStr);
  const mm = mmStr ? Number(mmStr) : 0;
  const ap = apRaw.toLowerCase();
  if (hh === 12) hh = ap === "am" ? 0 : 12;
  else if (ap === "pm") hh += 12;

  let dt: Date;
  if (month != null && day != null && Number.isFinite(day)) {
    dt = new Date(year, month, day, hh, mm, 0, 0);
    // If already past (by hours), assume next year.
    if (dt.getTime() < now.getTime() - 60_000) {
      dt = new Date(year + 1, month, day, hh, mm, 0, 0);
    }
  } else {
    dt = new Date(year, now.getMonth(), now.getDate(), hh, mm, 0, 0);
    if (dt.getTime() < now.getTime() - 60_000) {
      dt = new Date(year, now.getMonth(), now.getDate() + 1, hh, mm, 0, 0);
    }
  }

  return dt.toISOString();
}
