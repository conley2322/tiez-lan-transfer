import { expect, test } from "vitest";
import { hasSensitiveTag, isSensitiveTagName } from "./sensitiveTags";

test("sensitive tag names match the backend's, in any case", () => {
    for (const name of ["sensitive", "密码", "password", "Password", " SENSITIVE "]) {
        expect(isSensitiveTagName(name)).toBe(true);
    }
    expect(isSensitiveTagName("passwords")).toBe(false);
    expect(hasSensitiveTag(["work", "PASSWORD"])).toBe(true);
    expect(hasSensitiveTag(undefined)).toBe(false);
});
