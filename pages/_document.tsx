import Document, {Head, Html, Main, NextScript} from 'next/document';
import {createGetInitialProps} from '@mantine/next';

const getInitialProps = createGetInitialProps();

export default class _Document extends Document {
    static getInitialProps = getInitialProps;

    render(): JSX.Element {
        return (
            <Html lang={"ro"}>
                <Head>
                    <link rel="icon" href="/favicon.ico"/>
                    <link rel="manifest" href="/manifest.json" />
                    <link rel="apple-touch-icon" href="/icon.png"/>
                    {/*<meta name="theme-color" content="#fff" />*/}

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
