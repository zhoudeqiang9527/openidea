import { describe, expect, it } from "vitest";
import { projectColorIndex, projectColorVars, projectInitials } from "./projectColor";

describe("projectInitials", () => {
  it("follows IDEA's AvatarUtils.initials", () => {
    expect(projectInitials("idea")).toBe("I");
    expect(projectInitials("MyProject")).toBe("MP");
    expect(projectInitials("OpenIDEA")).toBe("OI");
    expect(projectInitials("py-demo")).toBe("PD");
    expect(projectInitials("web demo")).toBe("WD");
    expect(projectInitials("a.b.c")).toBe("AC");
    expect(projectInitials("123abc")).toBe("A");
    expect(projectInitials("")).toBe("?");
  });
});

describe("project colors", () => {
  it("uses the project path consistently for identically named projects", () => {
    const first = "E:/one/gupiao";
    const second = "E:/two/gupiao";
    expect(projectColorIndex(first)).not.toBe(projectColorIndex(second));
    expect(projectColorVars(first)).toEqual(projectColorVars(first));
    expect(projectColorVars(first)).not.toEqual(projectColorVars(second));
  });
});
