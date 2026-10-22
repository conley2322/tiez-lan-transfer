import { expect, test } from "vitest";
import { buildPingUrl } from "./useAnnouncements";

const PING = "https://tiez.name666.top/api/v1/ping";

test("the ping carries the startup ping's id", () => {
  const url = new URL(buildPingUrl(PING, "1.0.0-beta.8", "3fa91c0e"));

  expect(url.origin + url.pathname).toBe(PING);
  expect(url.searchParams.get("id")).toBe("3fa91c0e");
  expect(url.searchParams.get("v")).toBe("1.0.0-beta.8");
});

test("without an id the ping still fetches announcements, uncounted", () => {
  const url = new URL(buildPingUrl(PING, "1.0.0", null));

  expect(url.searchParams.has("id")).toBe(false);
  expect(url.searchParams.get("v")).toBe("1.0.0");
});

test("a ping URL that already has a query keeps it", () => {
  const url = new URL(buildPingUrl(`${PING}?channel=beta`, "1.0.0", "3fa91c0e"));

  expect(url.searchParams.get("channel")).toBe("beta");
  expect(url.searchParams.get("id")).toBe("3fa91c0e");
});
