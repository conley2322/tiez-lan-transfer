import { describe, it, expect } from "vitest";
import { formatSensitivePreview, getConciseTime } from "./utils";

describe("formatSensitivePreview", () => {
  describe("空值处理", () => {
    it("空字符串返回空", () => {
      expect(formatSensitivePreview("", "text")).toBe("");
    });
  });

  describe("普通文本掩码", () => {
    it("长文本保留前3后3", () => {
      const result = formatSensitivePreview("HelloWorld123", "text");
      expect(result).toBe("Hel...123");
    });

    it("极短文本(<=2字符)全部遮蔽", () => {
      expect(formatSensitivePreview("ab", "text")).toBe("...");
      expect(formatSensitivePreview("a", "text")).toBe("...");
    });

    it("3字符文本按比例分配可见字符", () => {
      // 3 chars, available = 1, totalRequested = 6 > 1
      // prefix = floor(1*3/6) = 0, suffix = min(3, 1) = 1
      const result = formatSensitivePreview("abc", "text");
      expect(result).toBe("...c");
    });

    it("自定义 prefix/suffix 可见数", () => {
      const result = formatSensitivePreview("1234567890", "text", {
        prefixVisible: 2,
        suffixVisible: 2,
      });
      expect(result).toBe("12...90");
    });

    it("prefix+suffix 超过可用字符时按比例截断", () => {
      // 5 chars, available = 3, totalRequested = 20 > 3
      // prefix = floor(3*10/20) = 1, suffix = min(10, 2) = 2
      const result = formatSensitivePreview("abcde", "text", {
        prefixVisible: 10,
        suffixVisible: 10,
      });
      expect(result).toBe("a...de");
    });

    it("suffixVisible=0 时所有可见预算分配给 prefix", () => {
      // 4 chars, available = 2, totalRequested = 2 <= 2
      // prefix = 2, suffix = 0
      const result = formatSensitivePreview("abcd", "text", {
        prefixVisible: 2,
        suffixVisible: 0,
      });
      expect(result).toBe("ab...");
    });

    it("prefixVisible=0 时所有可见预算分配给 suffix", () => {
      // 6 chars, available = 4, totalRequested = 3 <= 4
      // prefix = 0, suffix = 3
      const result = formatSensitivePreview("abcdef", "text", {
        prefixVisible: 0,
        suffixVisible: 3,
      });
      expect(result).toBe("...def");
    });
  });

  describe("URL 掩码", () => {
    it("保留协议头，遮蔽其余部分", () => {
      const result = formatSensitivePreview(
        "https://www.example.com/path",
        "url"
      );
      expect(result).toMatch(/^https:\/\//);
      expect(result).toContain("...");
      expect(result).not.toBe("https://www.example.com/path");
    });

    it("http 协议同样保留", () => {
      const result = formatSensitivePreview(
        "http://secret-server.internal:8080/api",
        "url"
      );
      expect(result).toMatch(/^http:\/\//);
      expect(result).toContain("...");
    });

    it("自定义协议也能识别", () => {
      const result = formatSensitivePreview(
        "ftp://files.example.com/doc.pdf",
        "url"
      );
      expect(result).toMatch(/^ftp:\/\//);
      expect(result).toContain("...");
    });

    it("无协议的 URL 按普通文本掩码", () => {
      const result = formatSensitivePreview("www.example.com", "url");
      expect(result).toBe("www...com");
    });
  });

  describe("邮箱掩码", () => {
    it("默认只遮蔽本地部分，保留域名", () => {
      const result = formatSensitivePreview(
        "username@example.com",
        "text"
      );
      expect(result).toContain("@example.com");
      expect(result).toContain("...");
      expect(result).not.toContain("username");
    });

    it("开启 maskEmailDomain 时域名也遮蔽", () => {
      const result = formatSensitivePreview(
        "username@example.com",
        "text",
        { maskEmailDomain: true }
      );
      expect(result).toContain("@");
      expect(result).not.toContain("example.com");
    });

    it("带空格的邮箱也能 trim 后识别", () => {
      const result = formatSensitivePreview(
        "  user@test.org  ",
        "text"
      );
      expect(result).toContain("@test.org");
      expect(result).toContain("...");
    });

    it("短本地部分(<=2字符)全部遮蔽", () => {
      const result = formatSensitivePreview("ab@example.com", "text");
      expect(result).toBe("...@example.com");
    });
  });

  describe("Unicode 支持", () => {
    it("中文字符按字符而非字节计数", () => {
      const result = formatSensitivePreview("你好世界测试文本", "text");
      expect(result).toContain("...");
      // 8 chars, available = 6, prefix = min(3,3) = 3, suffix = min(3,3) = 3
      expect(result).toBe("你好世...试文本");
    });

    it("emoji 按字符计数", () => {
      const result = formatSensitivePreview("😀😁😂🤣😃😄😅", "text");
      expect(result).toContain("...");
    });
  });
});

describe("getConciseTime", () => {
  it("空值或非法时间戳返回空字符串", () => {
    expect(getConciseTime(0, "zh")).toBe("");
    expect(getConciseTime(NaN, "zh")).toBe("");
  });

  it("1分钟内或轻微时钟偏差返回'刚刚' / 'Just now' / '剛剛'", () => {
    const now = Date.now();
    expect(getConciseTime(now - 10_000, "zh")).toBe("刚刚");
    expect(getConciseTime(now - 10_000, "en")).toBe("Just now");
    expect(getConciseTime(now - 10_000, "tw")).toBe("剛剛");
    expect(getConciseTime(now + 5_000, "zh")).toBe("刚刚");
  });

  it("1小时内返回相对分钟", () => {
    const now = Date.now();
    expect(getConciseTime(now - 15 * 60 * 1000, "zh")).toBe("15分钟前");
    expect(getConciseTime(now - 15 * 60 * 1000, "en")).toBe("15m ago");
    expect(getConciseTime(now - 15 * 60 * 1000, "tw")).toBe("15分鐘前");
  });

  it("当天超过1小时显示具体时间 HH:mm", () => {
    const d = new Date();
    d.setHours(14, 30, 0, 0);
    // If current time is 14:30, set d to 2 hours ago if it's afternoon, or test against fixed date
    const pad = (n: number) => n.toString().padStart(2, "0");
    const expected = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
    // As long as it's > 1h ago and same day
    const twoHoursAgo = Date.now() - 2 * 3600 * 1000;
    const twoHoursAgoDate = new Date(twoHoursAgo);
    if (twoHoursAgoDate.getDate() === new Date().getDate()) {
      const h = pad(twoHoursAgoDate.getHours());
      const m = pad(twoHoursAgoDate.getMinutes());
      expect(getConciseTime(twoHoursAgo, "zh")).toBe(`${h}:${m}`);
    }
  });

  it("昨天显示昨天和时分", () => {
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    yesterday.setHours(9, 15, 0, 0);
    expect(getConciseTime(yesterday.getTime(), "zh")).toBe("昨天 09:15");
    expect(getConciseTime(yesterday.getTime(), "en")).toBe("Yesterday 09:15");
  });

  it("今年其他日期显示 MM-DD", () => {
    const pastDate = new Date();
    // 30 days ago, making sure it stays in current year
    pastDate.setMonth(0, 15); // Jan 15 of this year
    pastDate.setHours(10, 0, 0, 0);
    if (new Date().getMonth() > 0 || new Date().getDate() > 16) {
      expect(getConciseTime(pastDate.getTime(), "zh")).toBe("01-15");
    }
  });

  it("跨年显示完整年月日 YYYY-MM-DD", () => {
    const pastYear = new Date();
    pastYear.setFullYear(2023, 4, 20); // 2023-05-20
    expect(getConciseTime(pastYear.getTime(), "zh")).toBe("2023-05-20");
    expect(getConciseTime(pastYear.getTime(), "en")).toBe("2023-05-20");
  });

  it("支持以秒为单位的时间戳", () => {
    const seconds = Math.floor(Date.now() / 1000) - 20;
    expect(getConciseTime(seconds, "zh")).toBe("刚刚");
  });
});
