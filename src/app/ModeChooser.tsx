import {
  CloudArrowUpRegular,
  LockClosedRegular,
  MicPulseRegular,
  Speaker2Regular,
} from "@fluentui/react-icons";

export type ApplicationMode = "cloud" | "local";

interface ModeChooserProps {
  onSelect: (mode: ApplicationMode) => void;
}

export function ModeChooser({ onSelect }: ModeChooserProps) {
  return (
    <main className="mode-chooser">
      <section className="mode-chooser-card" aria-labelledby="mode-chooser-title">
        <div className="mode-chooser-brand" aria-hidden="true">
          <MicPulseRegular fontSize={28} />
        </div>
        <div className="mode-chooser-heading">
          <span>Baka Trans</span>
          <h1 id="mode-chooser-title">Choose how to translate</h1>
          <p>
            Your audio devices and routing stay the same. Choose the translation engine for this
            session.
          </p>
        </div>

        <div className="mode-options">
          <button className="mode-option cloud" type="button" onClick={() => onSelect("cloud")}>
            <span className="mode-option-icon" aria-hidden="true">
              <CloudArrowUpRegular fontSize={24} />
            </span>
            <span className="mode-option-copy">
              <strong>Cloud API</strong>
              <span>Open the current Google and OpenAI workspace with all existing features.</span>
              <small>Requires a configured cloud API key</small>
            </span>
          </button>

          <button className="mode-option local" type="button" onClick={() => onSelect("local")}>
            <span className="mode-option-icon" aria-hidden="true">
              <Speaker2Regular fontSize={24} />
            </span>
            <span className="mode-option-copy">
              <strong>Local Whisper</strong>
              <span>Whisper speech-to-text, Gemma translation, then a local system voice.</span>
              <small>
                <LockClosedRegular fontSize={13} /> Audio and translation stay on this computer
              </small>
            </span>
          </button>
        </div>
      </section>
    </main>
  );
}
