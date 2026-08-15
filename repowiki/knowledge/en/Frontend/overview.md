# Frontend overview

The frontend is a React 18 renderer bootstrapped in `src/main.tsx`. `App.tsx` resolves the main, transparent overlay, and Look & Help routes; `MainApp.tsx` owns the principal desktop workspace. It delegates native work to `src/api.ts` and translates native DTOs through `src/types.ts`.

It owns user interaction, event subscriptions, transcript display/export shaping, local UI preferences, and settings presentation. It does not own audio devices, credentials, native windows, or provider network calls.
