import '../styles/globals.css'
import '@mantine/core/styles.css';
import '@mantine/dates/styles.css';
import Head from 'next/head';
import MyAppShell from "../components/AppShell";
import React, {ReactElement} from "react";
import {createTheme, MantineProvider} from '@mantine/core';
import {AppProps} from "next/app";
import {createPagesBrowserClient} from '@supabase/auth-helpers-nextjs'
import {Session, SessionContextProvider} from '@supabase/auth-helpers-react'
import {QueryClient, QueryClientProvider} from "react-query";
import {ProfileProvider} from "../hooks/useProfileData";

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {retry: 1}
    }
})

const theme = createTheme({
    primaryColor: 'orange',
    colors: {
        dark: [
            '#C1C2C5',
            '#A6A7AB',
            '#909296',
            '#5c5f66',
            '#373A40',
            '#2C2E33',
            '#25262b',
            '#1A1B1E',
            '#141517',
            '#101113',
        ],
    }
});

export default function MyApp({
                                  Component,
                                  pageProps,
                              }: AppProps<{
    initialSession: Session,
}>): ReactElement {
    const [supabaseClient] = React.useState(() => createPagesBrowserClient())

    return <>
        <Head>
            <title>ACSPA</title>
            <meta name="viewport" content="minimum-scale=1, initial-scale=1, width=device-width"/>
            <meta name="description" content="Site pentru membrii Asociației CS Perpetuum Activ"/>
        </Head>

        <QueryClientProvider client={queryClient}>
            <MantineProvider
                defaultColorScheme={'dark'}
                theme={theme}>
                <SessionContextProvider supabaseClient={supabaseClient}
                                        initialSession={pageProps.initialSession}>
                    <ProfileProvider>
                        <style jsx global>{`
                          html {
                            font-family: 'Open Sans', sans-serif;
                          }
                        `}</style>
                        <MyAppShell>
                            <Component {...pageProps} />
                        </MyAppShell>
                    </ProfileProvider>
                </SessionContextProvider>
            </MantineProvider>
        </QueryClientProvider>
    </>;
}
