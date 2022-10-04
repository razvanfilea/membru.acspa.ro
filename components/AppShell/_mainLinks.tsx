import React from 'react';
import {MdAccountBox, MdBookmarks, MdHome} from 'react-icons/md';
import {Avatar, Group, Text, ThemeIcon, UnstyledButton} from '@mantine/core';
import styles from './MyAppShell.module.css'
import Link from "next/link";
import {useAuth} from "../AuthProvider";

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
                    height: '70px',
                    padding: theme.spacing.xs,
                    borderRadius: 16,
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[5] : theme.colors.gray[2],
                    },
                })}
            >
                <Group noWrap={true}>
                    <ThemeIcon radius={"md"} color={color} size="lg">
                        {icon}
                    </ThemeIcon>

                    <Text size="md" className={styles.mainLinksText}>{label}</Text>
                </Group>
            </UnstyledButton>
        </Link>
    );
}


function UserProfileLink(name: string | null) {
    return (
        <Link href={"/profile"} passHref={true} prefetch={false}>
            <UnstyledButton
                sx={(theme) => ({
                    display: 'block',
                    width: '100%',
                    height: '70px',
                    padding: theme.spacing.xs,
                    borderRadius: 16,
                    backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
                    color: theme.colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,

                    '&:hover': {
                        backgroundColor:
                            theme.colorScheme === 'dark' ? theme.colors.dark[5] : theme.colors.gray[2],
                    },
                })}
            >
                {name != null &&
                    <Group noWrap={true}>
                        <>
                            <Avatar src={`https://ui-avatars.com/api/?name=${name}&background=random`}
                                    radius={"md"}/>

                            <Text size="md">{name}</Text>
                        </>
                    </Group>
                }
            </UnstyledButton>
        </Link>
    );
}

const data = [
    {icon: <MdHome size={22}/>, color: 'blue', label: 'Acasă', link: 'https://acspa.ro'},
    {icon: <MdBookmarks size={22}/>, color: 'green', label: 'Rezervări', link: '/'},
];

export default function MainLinks() {
    const auth = useAuth()

    const loginButtonData = {icon: <MdAccountBox size={22}/>, color: 'purple', label: 'Logare', link: '/login'}

    return <Group noWrap={true}>
        {data.map((link) => (
            <MainLink {...link} key={link.label}/>
        ))}

        {auth.user != null &&
            UserProfileLink(auth.profile?.name || "Profil")
        }

        {(!auth.isLoading && auth.user == null) &&
            <MainLink {...loginButtonData} />
        }
    </Group>;
}
