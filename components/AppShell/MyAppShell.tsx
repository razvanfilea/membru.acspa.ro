import React, {ReactElement, useEffect, useState} from 'react';
import {ActionIcon, AppShell, Burger, Group, Image, Transition, useMantineTheme} from '@mantine/core';
import MainLinks from "./_mainLinks";
import UserProfile from "./_user";
import {useRouter} from "next/router";
import {useMediaQuery} from "@mantine/hooks";
import HelpButton from "./_help";
import {MdGavel} from "react-icons/md";

export default function MyAppShell({children}): ReactElement {
    const router = useRouter()
    const theme = useMantineTheme()
    const [opened, setOpened] = useState(false)
    const navbarQuery = useMediaQuery('(min-width: 800px)', false, {getInitialValueInEffect: false});

    useEffect(() => {
        router.events.on("routeChangeComplete", () => {
            setOpened(false)
        });
    }, [router.events]);

    return <AppShell header={{height: 70}} navbar={{breakpoint: "sm", width: 300}}>
        <AppShell.Header p="sm" style={{display: 'flex', alignItems: 'center'}}>
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

            <Group style={{marginLeft: 'auto'}}>
                <ActionIcon
                    variant="filled"
                    radius={'md'}
                    size={'lg'}
                    onClick={() => window.open('regulament_intern.pdf', '_blank')}
                    color={'grape'}
                    title="Regulament Intern"
                >
                    <MdGavel size={18}/>
                </ActionIcon>

                <HelpButton/>
            </Group>
        </AppShell.Header>

        <Transition
            transition='slide-right' duration={300} timingFunction='ease'
            mounted={opened || navbarQuery || false}>
            {(styles) =>
                <AppShell.Navbar p="xs" style={styles}>
                    <MainLinks/>
                    <UserProfile/>
                </AppShell.Navbar>
            }
        </Transition>

        <AppShell.Main>
            {children}
        </AppShell.Main>
    </AppShell>
}
