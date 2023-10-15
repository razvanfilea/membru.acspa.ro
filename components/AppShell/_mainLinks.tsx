import React, {useMemo} from 'react';
import {MdAdminPanelSettings, MdBookmarks, MdHome} from 'react-icons/md';
import {Button, Stack, Text, ThemeIcon} from '@mantine/core';
import Link from "next/link";
import {MemberTypes} from "../../types/wrapper";
import useProfileData, {ProfileData} from "../../hooks/useProfileData";

interface MainLinkProps {
    icon: React.ReactNode;
    color: string;
    label: string;
    link: string;
}

function MainLink({icon, color, label, link}: MainLinkProps) {
    return <Link href={link} passHref={true} prefetch={false}>
        <Button
            leftSection={
                <ThemeIcon radius={"md"} variant={'light'} color={color} size="lg">
                    {icon}
                </ThemeIcon>
            }
            fullWidth={true}
            p={'xs'}
            radius={'sm'}
            size={'xl'}
            variant={'subtle'}
            color={'gray'}
            justify={'start'}
        >
            <Text size="md" c="#FFF">{label}</Text>
        </Button>
    </Link>
}

interface MainLinkData {
    icon: React.ReactNode;
    color: string;
    label: string;
    link: string;
    cond?: (auth: ProfileData) => boolean;
}

const linkData: MainLinkData[] = [
    {icon: <MdBookmarks size={22}/>, color: 'green', label: 'Rezervări', link: '/'},
    {icon: <MdHome size={22}/>, color: 'blue', label: 'Site ACSPA', link: 'https://acspa.ro'},
    {
        icon: <MdAdminPanelSettings size={22}/>,
        color: 'red',
        label: 'Panou Administrare',
        link: '/admin',
        cond: (profileData: ProfileData) => profileData.profile?.role === MemberTypes.Fondator
    },
];

export default function MainLinks() {
    const profileData = useProfileData()

    const links = useMemo(() => {
        return linkData.map((link) => {
            if (link.cond == null || link.cond(profileData)) {
                return <MainLink {...link} key={link.label}/>
            }
            return <React.Fragment key={link.label}/>
        })
    }, [profileData])

    return <Stack py={'sm'} gap={'xs'}>{links}</Stack>
}
