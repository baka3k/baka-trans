import { Tab, TabList, type SelectTabEvent, type SelectTabData } from "@fluentui/react-components";
import {
  BotRegular,
  BrainCircuitRegular,
  LiveRegular,
  SineWaveDotsRegular,
  TranslateRegular,
} from "@fluentui/react-icons";
import { useEffect, useState } from "react";

export type SettingsSection = "live" | "audio" | "translation" | "local_llm" | "summary";

interface AppNavigationProps {
  activeSection: SettingsSection;
  experience?: "cloud" | "local";
  onSelect: (section: SettingsSection) => void;
}

const destinations = [
  { value: "live", label: "Live", icon: <LiveRegular fontSize={20} /> },
  { value: "audio", label: "Audio", icon: <SineWaveDotsRegular fontSize={20} /> },
  { value: "translation", label: "Translation", icon: <TranslateRegular fontSize={20} /> },
  { value: "local_llm", label: "Local LLM", icon: <BrainCircuitRegular fontSize={20} /> },
  { value: "summary", label: "Summary", icon: <BotRegular fontSize={20} /> },
] satisfies Array<{ value: SettingsSection; label: string; icon: React.ReactElement }>;

export function AppNavigation({ activeSection, experience = "cloud", onSelect }: AppNavigationProps) {
  const [compact, setCompact] = useState(() => window.matchMedia("(max-width: 1040px)").matches);

  useEffect(() => {
    const media = window.matchMedia("(max-width: 1040px)");
    const updateLayout = (event: MediaQueryListEvent) => setCompact(event.matches);
    media.addEventListener("change", updateLayout);
    return () => media.removeEventListener("change", updateLayout);
  }, []);

  const handleSelect = (_event: SelectTabEvent, data: SelectTabData) => {
    onSelect(data.value as SettingsSection);
  };

  const visibleDestinations = destinations.filter((destination) =>
    experience === "local"
      ? ["live", "audio", "local_llm"].includes(destination.value)
      : destination.value !== "local_llm",
  );

  return (
    <nav className="app-navigation" aria-label="Workspace">
      <TabList
        appearance="subtle"
        vertical={!compact}
        selectedValue={activeSection}
        onTabSelect={handleSelect}
      >
        {visibleDestinations.map((destination) => (
          <Tab
            icon={destination.icon}
            key={destination.value}
            value={destination.value}
            aria-current={activeSection === destination.value ? "page" : undefined}
          >
            {destination.label}
          </Tab>
        ))}
      </TabList>
    </nav>
  );
}
