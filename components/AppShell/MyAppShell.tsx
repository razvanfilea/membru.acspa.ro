import React, {useEffect, useState} from 'react';
import {AppShell, Burger, Header, Image, Navbar, Transition, useMantineTheme} from '@mantine/core';
import LightAndDarkModeButton from "../LightAndDarkModeButton";
import MainLinks from "./_mainLinks";
import UserProfile from "./_user";
import {useRouter} from "next/router";
import {useMediaQuery} from "@mantine/hooks";

export default function MyAppShell({children}): JSX.Element {
    const router = useRouter()
    const theme = useMantineTheme()
    const [opened, setOpened] = useState(false)
    const navbarQuery = useMediaQuery('(min-width: 800px)', false, {getInitialValueInEffect: false});

    useEffect(() => {
        router.events.on("routeChangeComplete", () => {
            setOpened(false)
        });
    }, [router.events]);

    return <AppShell
        navbarOffsetBreakpoint="sm"
        header={
            <Header height={70} p="md">
                <div style={{display: 'flex', alignItems: 'center', height: '100%'}}>
                    {!navbarQuery &&
                        <Burger
                            opened={opened}
                            onClick={() => setOpened((o) => !o)}
                            size="sm"
                            color={theme.colors.gray[6]}
                            mr="xl"
                            title="Open menu"
                        />
                    }

                    <div>
                        <Image src={"/logo.webp"}
                               height={55}
                               fit={'contain'}
                               alt="Logo"/>
                    </div>

                    <div style={{marginLeft: 'auto'}}>
                        <LightAndDarkModeButton/>
                    </div>

                </div>
            </Header>
        }
        navbar={
            <Transition transition='slide-right' duration={300} timingFunction='ease' mounted={opened || navbarQuery}>
                {(styles) =>
                    <Navbar style={styles} p="xs" width={{base: 300}}>
                        <Navbar.Section mt="md">
                            <MainLinks/>
                        </Navbar.Section>
                        <Navbar.Section mt="md">
                            <UserProfile/>
                        </Navbar.Section>
                    </Navbar>
                }
            </Transition>
        }
    >
        {children}
    </AppShell>
}
