import React, {useEffect, useState} from 'react';
import {AppShell, Burger, Header, Image, MediaQuery, Navbar, useMantineTheme} from '@mantine/core';
import LightAndDarkModeButton from "../LightAndDarkModeButton";
import MainLinks from "./_mainLinks";
import UserProfile from "./_user";
import {router} from "next/client";
import {useRouter} from "next/router";

export default function MyAppShell({children}): JSX.Element {
    const router = useRouter()
    const theme = useMantineTheme()
    const [opened, setOpened] = useState(false)

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
                    <MediaQuery largerThan="sm" styles={{ display: 'none' }}>
                        <Burger
                            opened={opened}
                            onClick={() => setOpened((o) => !o)}
                            size="sm"
                            color={theme.colors.gray[6]}
                            mr="xl"
                        />
                    </MediaQuery>

                    <div>
                        <Image src={"https://acspa.ro/wp-content/uploads/2020/04/cropped-ACS-dd-oval-400-190x127.png"}
                               height={55}
                               fit={'contain'}
                               alt="ACS Perpetuum Activ"/>
                    </div>

                    <div style={{marginLeft: 'auto'}}>
                        <LightAndDarkModeButton/>
                    </div>

                </div>
            </Header>
        }
        navbar={
            <Navbar hiddenBreakpoint="sm" p="xs" hidden={!opened} width={{ base: 300 }}>
                <Navbar.Section mt="md">
                    <MainLinks />
                </Navbar.Section>
                <Navbar.Section mt="md">
                    <UserProfile />
                </Navbar.Section>
            </Navbar>
        }
    >
        {children}
    </AppShell>
}
