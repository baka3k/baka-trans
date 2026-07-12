import {
  createDarkTheme,
  createLightTheme,
  type BrandVariants,
  type Theme,
} from "@fluentui/react-components";

export const bakaBrand: BrandVariants = {
  10: "#031F12",
  20: "#07351E",
  30: "#0A4A29",
  40: "#0C6035",
  50: "#0E7641",
  60: "#108C4D",
  70: "#13A259",
  80: "#18B768",
  90: "#36C77C",
  100: "#57D590",
  110: "#78E1A5",
  120: "#98EBBA",
  130: "#B7F3CF",
  140: "#D3F8E0",
  150: "#E9FCEF",
  160: "#F5FEF8",
};

const baseLightTheme = createLightTheme(bakaBrand);
const baseDarkTheme = createDarkTheme(bakaBrand);

export const lightTheme: Theme = {
  ...baseLightTheme,
  fontFamilyBase:
    '"Segoe UI Variable", "Segoe UI", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  colorBrandBackground: bakaBrand[70],
  colorBrandBackgroundHover: bakaBrand[60],
  colorBrandBackgroundPressed: bakaBrand[50],
  colorBrandForeground1: bakaBrand[60],
  colorBrandForeground2: bakaBrand[50],
};

export const darkTheme: Theme = {
  ...baseDarkTheme,
  fontFamilyBase:
    '"Segoe UI Variable", "Segoe UI", -apple-system, BlinkMacSystemFont, system-ui, sans-serif',
  colorBrandBackground: bakaBrand[90],
  colorBrandBackgroundHover: bakaBrand[100],
  colorBrandBackgroundPressed: bakaBrand[80],
  colorBrandForeground1: bakaBrand[100],
  colorBrandForeground2: bakaBrand[110],
};
