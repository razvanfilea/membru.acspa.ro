import Document, {Head, Html, Main, NextScript} from 'next/document';
import {createGetInitialProps} from '@mantine/next';
import React, {ReactElement} from "react";

const getInitialProps = createGetInitialProps();

export default class _Document extends Document {
    static getInitialProps = getInitialProps;

    render(): ReactElement {
        return (
            <Html lang={"ro"}>
                <Head>
                    <link rel="manifest" href="/manifest.json"/>
                    <link rel="apple-touch-icon" sizes="180x180" href="/fav/apple-touch-icon.png"/>
                    <link rel="icon" type="image/png" sizes="32x32" href="/fav/favicon-32x32.png"/>
                    <link rel="icon" type="image/ico" sizes="16x16" href="/fav/favicon-16x16.ico"/>
                    <link rel="manifest" href="/fav/site.webmanifest"/>

                    <link
                        href="https://fonts.googleapis.com/css2?family=Open+Sans:wght@400&display=swap"
                        rel="stylesheet"
                    />
                </Head>
                <body>
                <Main/>
                <NextScript/>
                </body>
            </Html>
        )
    }
}
