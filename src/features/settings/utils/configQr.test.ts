import { describe, expect, it } from "vitest";
import {
  buildMqttConfigQrPayload,
  buildWebdavConfigQrPayload,
  isMqttConfigQrReady,
  isWebdavConfigQrReady,
} from "./configQr";

describe("Settings configQr utility", () => {
  it("builds valid MQTT QR payload with default fallback paths", () => {
    const payload = buildMqttConfigQrPayload({
      server: " broker.emqx.io ",
      port: " 8084 ",
      protocol: "",
      wsPath: "",
      username: " user1 ",
      password: "secretpassword",
      topic: " tiez/sync/room1 ",
    });

    expect(payload.v).toBe(1);
    expect(payload.type).toBe("tiez-mqtt");
    expect(payload.mqtt.server).toBe("broker.emqx.io");
    expect(payload.mqtt.port).toBe("8084");
    expect(payload.mqtt.protocol).toBe("wss://");
    expect(payload.mqtt.wsPath).toBe("/mqtt");
    expect(payload.mqtt.topic).toBe("tiez/sync/room1");
    expect(payload.e2eKey).toBeUndefined();
    expect(isMqttConfigQrReady(payload)).toBe(true);
  });

  it("includes e2eKey in MQTT payload when specified", () => {
    const payload = buildMqttConfigQrPayload({
      server: "broker.example.com",
      port: "8883",
      protocol: "mqtts://",
      wsPath: "",
      username: "",
      password: "",
      topic: "my-topic",
      e2eKey: " e2e-secret-key-123 ",
    });

    expect(payload.e2eKey).toBe("e2e-secret-key-123");
  });

  it("correctly determines if MQTT QR payload is ready", () => {
    const notReady = buildMqttConfigQrPayload({
      server: "",
      port: "1883",
      protocol: "mqtt://",
      wsPath: "",
      username: "",
      password: "",
      topic: "",
    });
    expect(isMqttConfigQrReady(notReady)).toBe(false);
  });

  it("builds valid WebDAV QR payload", () => {
    const payload = buildWebdavConfigQrPayload({
      url: "https://dav.jianguoyun.com/dav/",
      username: "user@example.com",
      password: "app-password",
      basePath: "",
      e2eKey: "my-dav-key",
    });

    expect(payload.v).toBe(1);
    expect(payload.type).toBe("tiez-webdav");
    expect(payload.webdav.url).toBe("https://dav.jianguoyun.com/dav/");
    expect(payload.webdav.basePath).toBe("tiez-sync");
    expect(payload.e2eKey).toBe("my-dav-key");
    expect(isWebdavConfigQrReady(payload)).toBe(true);
  });

  it("rejects incomplete WebDAV QR payloads", () => {
    const emptyPayload = buildWebdavConfigQrPayload({
      url: "  ",
      username: "",
      password: "",
      basePath: "",
    });
    expect(isWebdavConfigQrReady(emptyPayload)).toBe(false);
  });
});
