import '../styles/globals.css'
import Head from 'next/head';
import MyAppShell from "../components/AppShell";
import React from "react";
import {ColorScheme, ColorSchemeProvider, MantineProvider, Paper} from '@mantine/core';
import {useLocalStorage} from "@mantine/hooks";
import AuthProvider from "../components/AuthProvider";

export default function MyApp({Component, pageProps}): JSX.Element {
    const [colorScheme, setLocalColorScheme] = useLocalStorage<ColorScheme>({
        key: 'color-scheme',
        defaultValue: 'dark',
        getInitialValueInEffect: true,
    });

    const toggleColorScheme = (value?: ColorScheme) => {
        const newValue = value || (colorScheme === 'dark' ? 'light' : 'dark')
        setLocalColorScheme(newValue)
    }

    return <>
        <Head>
            <title>ACSPA</title>
            <meta name="viewport" content="minimum-scale=1, initial-scale=1, width=device-width"/>
            <meta name="description" content="Site pentru membrii Asociației ACS Perpetuum Activ"/>
        </Head>

        <ColorSchemeProvider colorScheme={colorScheme} toggleColorScheme={toggleColorScheme}>

            <MantineProvider
                withGlobalStyles
                withNormalizeCSS
                theme={{
                    fontFamily: 'Open Sans',
                    colorScheme: colorScheme,
                    primaryColor: 'orange'
                }}
            >
                <Paper>
                    <AuthProvider>
                        <MyAppShell>
                            <Component {...pageProps} />
                        </MyAppShell>
                    </AuthProvider>
                </Paper>
            </MantineProvider>
        </ColorSchemeProvider>
    </>;
}
