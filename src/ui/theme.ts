import {
  createDarkTheme,
  createLightTheme,
  type BrandVariants,
  type Theme,
} from "@fluentui/react-components";

export const bakaBrand: BrandVariants = {
  10: "#041D19",
  20: "#073129",
  30: "#0A463A",
  40: "#0C5B4B",
  50: "#0E705C",
  60: "#10856D",
  70: "#15977C",
  80: "#20A98E",
  90: "#37B99D",
  100: "#55C6AD",
  110: "#75D2BD",
  120: "#94DDCC",
  130: "#B2E7DA",
  140: "#CDEFE6",
  150: "#E3F6F0",
  160: "#F1FBF7",
};

const baseLightTheme = createLightTheme(bakaBrand);
const baseDarkTheme = createDarkTheme(bakaBrand);

export const lightTheme: Theme = {
  ...baseLightTheme,
  fontFamilyBase:
    '"Segoe UI Variable", "Segoe UI", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  colorNeutralBackground1: "#FBFDFC",
  colorNeutralBackground2: "#F2F6F4",
  colorNeutralBackground3: "#E9EFEC",
  colorNeutralBackground4: "#E1E9E5",
  colorNeutralForeground1: "#14201C",
  colorNeutralForeground2: "#43534C",
  colorNeutralForeground3: "#63736C",
  colorNeutralStroke1: "#C8D4CF",
  colorNeutralStroke2: "#DCE4E0",
  colorBrandBackground: bakaBrand[50],
  colorBrandBackgroundHover: bakaBrand[40],
  colorBrandBackgroundPressed: bakaBrand[30],
  colorBrandForeground1: bakaBrand[50],
  colorBrandForeground2: bakaBrand[40],
};

export const darkTheme: Theme = {
  ...baseDarkTheme,
  fontFamilyBase:
    '"Segoe UI Variable", "Segoe UI", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  colorNeutralBackground1: "#1D2421",
  colorNeutralBackground2: "#151B18",
  colorNeutralBackground3: "#252E2A",
  colorNeutralBackground4: "#2D3732",
  colorNeutralForeground1: "#F1F6F3",
  colorNeutralForeground2: "#C2CEC8",
  colorNeutralForeground3: "#94A39C",
  colorNeutralForegroundOnBrand: "#071E19",
  colorNeutralStroke1: "#47564F",
  colorNeutralStroke2: "#34413B",
  colorBrandBackground: bakaBrand[90],
  colorBrandBackgroundHover: bakaBrand[100],
  colorBrandBackgroundPressed: bakaBrand[80],
  colorBrandForeground1: bakaBrand[100],
  colorBrandForeground2: bakaBrand[110],
};
