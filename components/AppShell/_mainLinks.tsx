import React, {useMemo} from 'react';
import {MdAdminPanelSettings, MdBookmarks, MdHome} from 'react-icons/md';
import {Group, Stack, Text, ThemeIcon, UnstyledButton} from '@mantine/core';
import Link from "next/link";
import {AuthData, useAuth} from "../AuthProvider";

interface MainLinkProps {
    icon: React.ReactNode;
    color: string;
    label: string;
    link: string;
}

function MainLink({icon, color, label, link}: MainLinkProps) {
    return (
        <Link href={link} passHref={true} prefetch={false}>
            <UnstyledButton
                sx={(theme) => ({
                    display: 'block',
                    width: '100%',
                    padding: theme.spacing.xs,
                    borderRadius: theme.radius.sm,
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[0],
                    },
                })}
            >
                <Group noWrap={true}>
                    <ThemeIcon radius={"md"} variant={'light'} color={color} size="lg">
                        {icon}
                    </ThemeIcon>

                    <Text size="md">{label}</Text>
                </Group>
            </UnstyledButton>
        </Link>
    );
}

interface MainLinkData {
    icon: React.ReactNode;
    color: string;
    label: string;
    link: string;
    cond?: (auth: AuthData) => boolean;
}

const linkData: MainLinkData[] = [
    {icon: <MdBookmarks size={22}/>, color: 'green', label: 'Rezervări', link: '/'},
    {icon: <MdHome size={22}/>, color: 'blue', label: 'Site ACSPA', link: 'https://acspa.ro'},
    {
        icon: <MdAdminPanelSettings size={22}/>,
        color: 'red',
        label: 'Admin',
        link: '/admin',
        cond: (auth: AuthData) => auth.profile?.member_type === 'Fondator'
    },
];

export default function MainLinks() {
    const auth = useAuth()

    const links = useMemo(() => {
        return linkData.map((link) => {
            if (link.cond == null || link.cond(auth)) {
                return <MainLink {...link} key={link.label}/>
            }
            return <React.Fragment key={link.label}/>
        })
    }, [auth])

    return <Stack>{links}</Stack>
}
