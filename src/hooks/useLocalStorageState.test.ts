import { afterEach, describe, expect, it } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useLocalStorageState } from "./useLocalStorageState";

describe("useLocalStorageState", () => {
  afterEach(() => {
    window.localStorage.clear();
  });

  it("initializes from the default value when nothing is stored", () => {
    const { result } = renderHook(() => useLocalStorageState("kunger.test.mode", "table"));

    expect(result.current[0]).toBe("table");
  });

  it("initializes from localStorage when a value is already stored", () => {
    window.localStorage.setItem("kunger.test.mode", JSON.stringify("grouped"));

    const { result } = renderHook(() => useLocalStorageState("kunger.test.mode", "table"));

    expect(result.current[0]).toBe("grouped");
  });

  it("persists updates to localStorage", () => {
    const { result } = renderHook(() => useLocalStorageState("kunger.test.mode", "table"));

    act(() => {
      result.current[1]("grouped");
    });

    expect(result.current[0]).toBe("grouped");
    expect(window.localStorage.getItem("kunger.test.mode")).toBe(JSON.stringify("grouped"));
  });

  it("falls back to the default value when the stored data is corrupt", () => {
    window.localStorage.setItem("kunger.test.mode", "{not valid json");

    const { result } = renderHook(() => useLocalStorageState("kunger.test.mode", "table"));

    expect(result.current[0]).toBe("table");
  });
});
