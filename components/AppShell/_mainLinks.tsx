import React from 'react';
import {MdBookmarks, MdHome} from 'react-icons/md';
import {Group, Stack, Text, ThemeIcon, UnstyledButton} from '@mantine/core';
import Link from "next/link";

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

const data = [
    {icon: <MdBookmarks size={22}/>, color: 'green', label: 'Rezervări', link: '/'},
    {icon: <MdHome size={22}/>, color: 'blue', label: 'Site ACSPA', link: 'https://acspa.ro'},
];

export default function MainLinks() {
    return <Stack>
        {data.map((link) => (
            <MainLink {...link} key={link.label}/>
        ))}
    </Stack>
}
