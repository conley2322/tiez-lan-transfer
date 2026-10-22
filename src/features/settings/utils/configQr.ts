export type TiezMqttQrPayload = {
  v: 1;
  type: "tiez-mqtt";
  mqtt: {
    server: string;
    port: string;
    protocol: string;
    wsPath: string;
    username: string;
    password: string;
    topic: string;
  };
  /** End-to-end key, when encryption is on (docs/E2E-ENCRYPTION.md). */
  e2eKey?: string;
};

export type TiezWebdavQrPayload = {
  v: 1;
  type: "tiez-webdav";
  webdav: {
    url: string;
    username: string;
    password: string;
    basePath: string;
  };
  e2eKey?: string;
};

export const buildMqttConfigQrPayload = (input: {
  server: string;
  port: string;
  protocol: string;
  wsPath: string;
  username: string;
  password: string;
  topic: string;
  e2eKey?: string;
}): TiezMqttQrPayload => ({
  v: 1,
  type: "tiez-mqtt",
  mqtt: {
    server: input.server.trim(),
    port: input.port.trim(),
    protocol: input.protocol.trim() || "wss://",
    wsPath: input.wsPath.trim() || "/mqtt",
    username: input.username.trim(),
    password: input.password,
    topic: input.topic.trim(),
  },
  ...(input.e2eKey?.trim() ? { e2eKey: input.e2eKey.trim() } : {}),
});

export const buildWebdavConfigQrPayload = (input: {
  url: string;
  username: string;
  password: string;
  basePath: string;
  e2eKey?: string;
}): TiezWebdavQrPayload => ({
  v: 1,
  type: "tiez-webdav",
  webdav: {
    url: input.url.trim(),
    username: input.username.trim(),
    password: input.password,
    basePath: input.basePath.trim() || "tiez-sync",
  },
  ...(input.e2eKey?.trim() ? { e2eKey: input.e2eKey.trim() } : {}),
});

export const isMqttConfigQrReady = (payload: TiezMqttQrPayload) =>
  payload.mqtt.server.length > 0 && payload.mqtt.topic.length > 0;

export const isWebdavConfigQrReady = (payload: TiezWebdavQrPayload) =>
  payload.webdav.url.length > 0;
