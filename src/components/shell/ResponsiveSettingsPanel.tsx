import { Button } from "@fluentui/react-components";
import { DismissRegular } from "@fluentui/react-icons";
import { useEffect, useRef, type PropsWithChildren, type RefObject } from "react";
import type { SettingsSection } from "./AppNavigation";

interface ResponsiveSettingsPanelProps extends PropsWithChildren {
  open: boolean;
  section: SettingsSection;
  onClose: () => void;
  returnFocusRef: RefObject<HTMLButtonElement | null>;
}

const focusableSelector = [
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[href]",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

function isVisible(element: HTMLElement) {
  for (let current: HTMLElement | null = element; current; current = current.parentElement) {
    const style = window.getComputedStyle(current);
    if (style.display === "none" || style.visibility === "hidden") {
      return false;
    }
  }
  return true;
}

function focusableElements(container: HTMLElement | null) {
  return Array.from(container?.querySelectorAll<HTMLElement>(focusableSelector) ?? []).filter(
    isVisible,
  );
}

export function ResponsiveSettingsPanel({
  children,
  open,
  section,
  onClose,
  returnFocusRef,
}: ResponsiveSettingsPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const title =
    section === "translation"
      ? "Translation settings"
      : section === "local_llm"
        ? "Local LLM settings"
        : `${section[0].toUpperCase()}${section.slice(1)} settings`;
  const modal = open && !window.matchMedia("(min-width: 1280px)").matches;

  useEffect(() => {
    if (!open || window.matchMedia("(min-width: 1280px)").matches) {
      return;
    }
    const first = focusableElements(panelRef.current)[0];
    first?.focus();
  }, [open, section]);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      returnFocusRef.current?.focus();
      return;
    }
    if (event.key !== "Tab" || window.matchMedia("(min-width: 1280px)").matches) {
      return;
    }
    const focusable = focusableElements(panelRef.current);
    if (focusable.length === 0) {
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <>
      {open ? <button className="settings-backdrop" aria-label="Close settings" onClick={onClose} /> : null}
      <div
        className="settings-column"
        id="session-settings"
        aria-label={title}
        aria-modal={modal ? "true" : undefined}
        role={modal ? "dialog" : "complementary"}
        data-open={open}
        data-section={section}
        hidden={!open}
        onKeyDown={handleKeyDown}
        ref={panelRef}
      >
        <div className="settings-panel-heading">
          <div>
            <span>Setup</span>
            <h2>{title}</h2>
          </div>
          <Button
            appearance="subtle"
            aria-label="Close settings"
            className="settings-panel-close"
            icon={<DismissRegular fontSize={20} />}
            onClick={() => {
              onClose();
              returnFocusRef.current?.focus();
            }}
          />
        </div>
        {children}
      </div>
    </>
  );
}
